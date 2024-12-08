use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use automation_lib::action_callback::ActionCallback;
use automation_lib::config::MqttDeviceConfig;
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::error::DeviceConfigError;
use automation_lib::event::{OnMqtt, OnPresence};
use automation_lib::messages::{ContactMessage, PresenceMessage};
use automation_lib::mqtt::WrappedAsyncClient;
use automation_lib::presence::DEFAULT_PRESENCE;
use automation_macro::LuaDeviceConfig;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::task::JoinHandle;
use tracing::{debug, error, trace, warn};

// NOTE: If we add more presence devices we might need to move this out of here
#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct PresenceDeviceConfig {
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(with(Duration::from_secs))]
    pub timeout: Duration,
}

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config {
    pub identifier: String,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(from_lua, default)]
    pub presence: Option<PresenceDeviceConfig>,
    #[device_config(from_lua, default)]
    pub callback: ActionCallback<ContactSensor, bool>,
    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

#[derive(Debug)]
struct State {
    overall_presence: bool,
    is_closed: bool,
    handle: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub struct ContactSensor {
    config: Config,
    state: Arc<RwLock<State>>,
}

impl ContactSensor {
    async fn state(&self) -> RwLockReadGuard<State> {
        self.state.read().await
    }

    async fn state_mut(&self) -> RwLockWriteGuard<State> {
        self.state.write().await
    }
}

#[async_trait]
impl LuaDeviceCreate for ContactSensor {
    type Config = Config;
    type Error = DeviceConfigError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.identifier, "Setting up ContactSensor");

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        let state = State {
            overall_presence: DEFAULT_PRESENCE,
            is_closed: true,
            handle: None,
        };
        let state = Arc::new(RwLock::new(state));

        Ok(Self { config, state })
    }
}

impl Device for ContactSensor {
    fn get_id(&self) -> String {
        self.config.identifier.clone()
    }
}

#[async_trait]
impl OnPresence for ContactSensor {
    async fn on_presence(&self, presence: bool) {
        self.state_mut().await.overall_presence = presence;
    }
}

#[async_trait]
impl OnMqtt for ContactSensor {
    async fn on_mqtt(&self, message: rumqttc::Publish) {
        if !rumqttc::matches(&message.topic, &self.config.mqtt.topic) {
            return;
        }

        let is_closed = match ContactMessage::try_from(message) {
            Ok(state) => state.is_closed(),
            Err(err) => {
                error!(id = self.get_id(), "Failed to parse message: {err}");
                return;
            }
        };

        if is_closed == self.state().await.is_closed {
            return;
        }

        self.config.callback.call(self, &!is_closed).await;

        debug!(id = self.get_id(), "Updating state to {is_closed}");
        self.state_mut().await.is_closed = is_closed;

        // Check if this contact sensor works as a presence device
        // If not we are done here
        let presence = match &self.config.presence {
            Some(presence) => presence.clone(),
            None => return,
        };

        if !is_closed {
            // Activate presence and stop any timeout once we open the door
            if let Some(handle) = self.state_mut().await.handle.take() {
                handle.abort();
            }

            // Only use the door as an presence sensor if there the current presence is set false
            // This is to prevent the house from being marked as present for however long the
            // timeout is set when leaving the house
            if !self.state().await.overall_presence {
                self.config
                    .client
                    .publish(
                        &presence.mqtt.topic,
                        rumqttc::QoS::AtLeastOnce,
                        false,
                        serde_json::to_string(&PresenceMessage::new(true)).unwrap(),
                    )
                    .await
                    .map_err(|err| {
                        warn!(
                            "Failed to publish presence on {}: {err}",
                            presence.mqtt.topic
                        )
                    })
                    .ok();
            }
        } else {
            // Once the door is closed again we start a timeout for removing the presence
            let device = self.clone();
            self.state_mut().await.handle = Some(tokio::spawn(async move {
                debug!(
                    id = device.get_id(),
                    "Starting timeout ({:?}) for contact sensor...", presence.timeout
                );
                tokio::time::sleep(presence.timeout).await;
                debug!(id = device.get_id(), "Removing door device!");
                device
                    .config
                    .client
                    .publish(&presence.mqtt.topic, rumqttc::QoS::AtLeastOnce, false, "")
                    .await
                    .map_err(|err| {
                        warn!(
                            "Failed to publish presence on {}: {err}",
                            presence.mqtt.topic
                        )
                    })
                    .ok();
            }));
        }
    }
}
