use async_trait::async_trait;
use rumqttc::{AsyncClient, Publish};
use serde::Deserialize;
use tracing::{debug, error, warn};

use crate::{
    config::{CreateDevice, MqttDeviceConfig},
    device_manager::DeviceManager,
    error::CreateDeviceError,
    event::{Event, EventChannel, OnMqtt},
    messages::PowerMessage,
};

use super::{ntfy::Priority, Device, Notification};

#[derive(Debug, Clone, Deserialize)]
pub struct WasherConfig {
    #[serde(flatten)]
    mqtt: MqttDeviceConfig,
    threshold: f32, // Power in Watt
}

// TODO: Add google home integration

#[derive(Debug)]
pub struct Washer {
    identifier: String,
    mqtt: MqttDeviceConfig,

    event_channel: EventChannel,
    threshold: f32,
    running: isize,
}

#[async_trait]
impl CreateDevice for Washer {
    type Config = WasherConfig;

    async fn create(
        identifier: &str,
        config: Self::Config,
        event_channel: &EventChannel,
        _client: &AsyncClient,
        _presence_topic: &str,
        _device_manager: &DeviceManager,
    ) -> Result<Self, CreateDeviceError> {
        Ok(Self {
            identifier: identifier.to_owned(),
            mqtt: config.mqtt,
            event_channel: event_channel.clone(),
            threshold: config.threshold,
            running: 0,
        })
    }
}

impl Device for Washer {
    fn get_id(&self) -> &str {
        &self.identifier
    }
}

// The washer needs to have a power draw above the theshold multiple times before the washer is
// actually marked as running
// This helps prevent false positives
const HYSTERESIS: isize = 3;

#[async_trait]
impl OnMqtt for Washer {
    fn topics(&self) -> Vec<&str> {
        vec![&self.mqtt.topic]
    }

    async fn on_mqtt(&mut self, message: Publish) {
        let power = match PowerMessage::try_from(message) {
            Ok(state) => state.power(),
            Err(err) => {
                error!(id = self.identifier, "Failed to parse message: {err}");
                return;
            }
        };

        debug!(id = self.identifier, power, "Washer state update");

        if power < self.threshold && self.running >= HYSTERESIS {
            // The washer is done running
            debug!(
                id = self.identifier,
                power,
                threshold = self.threshold,
                "Washer is done"
            );

            self.running = 0;
            let notification = Notification::new()
                .set_title("Laundy is done")
                .set_message("Don't forget to hang it!")
                .add_tag("womans_clothes")
                .set_priority(Priority::High);

            if self
                .event_channel
                .get_tx()
                .send(Event::Ntfy(notification))
                .await
                .is_err()
            {
                warn!("There are no receivers on the event channel");
            }
        } else if power < self.threshold {
            // Prevent false positives
            self.running = 0;
        } else if power >= self.threshold && self.running < HYSTERESIS {
            // Washer could be starting
            debug!(
                id = self.identifier,
                power,
                threshold = self.threshold,
                "Washer is starting"
            );

            self.running += 1;
        }
    }
}
