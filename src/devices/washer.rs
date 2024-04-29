use async_trait::async_trait;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use rumqttc::Publish;
use tracing::{debug, error, trace, warn};

use super::ntfy::Priority;
use super::{Device, LuaDeviceCreate, Notification};
use crate::config::MqttDeviceConfig;
use crate::event::{self, Event, EventChannel, OnMqtt};
use crate::messages::PowerMessage;
use crate::mqtt::WrappedAsyncClient;

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct WasherConfig {
    pub identifier: String,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    // Power in Watt
    pub threshold: f32,
    #[device_config(rename("event_channel"), from_lua, with(|ec: EventChannel| ec.get_tx()))]
    pub tx: event::Sender,
    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

// TODO: Add google home integration
#[derive(Debug, LuaDevice)]
pub struct Washer {
    config: WasherConfig,

    running: isize,
}

#[async_trait]
impl LuaDeviceCreate for Washer {
    type Config = WasherConfig;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.identifier, "Setting up Washer");

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        Ok(Self { config, running: 0 })
    }
}

impl Device for Washer {
    fn get_id(&self) -> String {
        self.config.identifier.clone()
    }
}

// The washer needs to have a power draw above the threshold multiple times before the washer is
// actually marked as running
// This helps prevent false positives
const HYSTERESIS: isize = 10;

#[async_trait]
impl OnMqtt for Washer {
    async fn on_mqtt(&mut self, message: Publish) {
        if !rumqttc::matches(&message.topic, &self.config.mqtt.topic) {
            return;
        }

        let power = match PowerMessage::try_from(message) {
            Ok(state) => state.power(),
            Err(err) => {
                error!(
                    id = self.config.identifier,
                    "Failed to parse message: {err}"
                );
                return;
            }
        };

        // debug!(id = self.identifier, power, "Washer state update");

        if power < self.config.threshold && self.running >= HYSTERESIS {
            // The washer is done running
            debug!(
                id = self.config.identifier,
                power,
                threshold = self.config.threshold,
                "Washer is done"
            );

            self.running = 0;
            let notification = Notification::new()
                .set_title("Laundy is done")
                .set_message("Don't forget to hang it!")
                .add_tag("womans_clothes")
                .set_priority(Priority::High);

            if self
                .config
                .tx
                .send(Event::Ntfy(notification))
                .await
                .is_err()
            {
                warn!("There are no receivers on the event channel");
            }
        } else if power < self.config.threshold {
            // Prevent false positives
            self.running = 0;
        } else if power >= self.config.threshold && self.running < HYSTERESIS {
            // Washer could be starting
            debug!(
                id = self.config.identifier,
                power,
                threshold = self.config.threshold,
                "Washer is starting"
            );

            self.running += 1;
        }
    }
}
