use std::sync::Arc;

use async_trait::async_trait;
use automation_lib::config::MqttDeviceConfig;
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::event::{self, Event, EventChannel, OnMqtt};
use automation_lib::messages::PowerMessage;
use automation_lib::mqtt::WrappedAsyncClient;
use automation_lib::ntfy::{Notification, Priority};
use automation_macro::{LuaDeviceConfig, impl_device};
use rumqttc::Publish;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, error, trace, warn};

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config {
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

#[derive(Debug)]
pub struct State {
    running: isize,
}

// TODO: Add google home integration
#[derive(Debug, Clone)]
pub struct Washer {
    config: Config,
    state: Arc<RwLock<State>>,
}
impl_device!(Washer);

impl Washer {
    async fn state(&self) -> RwLockReadGuard<'_, State> {
        self.state.read().await
    }

    async fn state_mut(&self) -> RwLockWriteGuard<'_, State> {
        self.state.write().await
    }
}

#[async_trait]
impl LuaDeviceCreate for Washer {
    type Config = Config;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.identifier, "Setting up Washer");

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        let state = State { running: 0 };
        let state = Arc::new(RwLock::new(state));

        Ok(Self { config, state })
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
    async fn on_mqtt(&self, message: Publish) {
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

        if power < self.config.threshold && self.state().await.running >= HYSTERESIS {
            // The washer is done running
            debug!(
                id = self.config.identifier,
                power,
                threshold = self.config.threshold,
                "Washer is done"
            );

            self.state_mut().await.running = 0;
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
            self.state_mut().await.running = 0;
        } else if power >= self.config.threshold && self.state().await.running < HYSTERESIS {
            // Washer could be starting
            debug!(
                id = self.config.identifier,
                power,
                threshold = self.config.threshold,
                "Washer is starting"
            );

            self.state_mut().await.running += 1;
        }
    }
}
