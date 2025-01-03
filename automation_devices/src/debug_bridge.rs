use std::convert::Infallible;

use async_trait::async_trait;
use automation_lib::config::MqttDeviceConfig;
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::event::{OnDarkness, OnPresence};
use automation_lib::messages::{DarknessMessage, PresenceMessage};
use automation_lib::mqtt::WrappedAsyncClient;
use automation_macro::LuaDeviceConfig;
use tracing::{trace, warn};

#[derive(Debug, LuaDeviceConfig, Clone)]
pub struct Config {
    pub identifier: String,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

#[derive(Debug, Clone)]
pub struct DebugBridge {
    config: Config,
}

#[async_trait]
impl LuaDeviceCreate for DebugBridge {
    type Config = Config;
    type Error = Infallible;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.identifier, "Setting up DebugBridge");
        Ok(Self { config })
    }
}

impl Device for DebugBridge {
    fn get_id(&self) -> String {
        self.config.identifier.clone()
    }
}

#[async_trait]
impl OnPresence for DebugBridge {
    async fn on_presence(&self, presence: bool) {
        let message = PresenceMessage::new(presence);
        let topic = format!("{}/presence", self.config.mqtt.topic);
        self.config
            .client
            .publish(
                topic,
                rumqttc::QoS::AtLeastOnce,
                true,
                serde_json::to_string(&message).expect("Serialization should not fail"),
            )
            .await
            .map_err(|err| {
                warn!(
                    "Failed to update presence on {}/presence: {err}",
                    self.config.mqtt.topic
                )
            })
            .ok();
    }
}

#[async_trait]
impl OnDarkness for DebugBridge {
    async fn on_darkness(&self, dark: bool) {
        let message = DarknessMessage::new(dark);
        let topic = format!("{}/darkness", self.config.mqtt.topic);
        self.config
            .client
            .publish(
                topic,
                rumqttc::QoS::AtLeastOnce,
                true,
                serde_json::to_string(&message).unwrap(),
            )
            .await
            .map_err(|err| {
                warn!(
                    "Failed to update presence on {}/presence: {err}",
                    self.config.mqtt.topic
                )
            })
            .ok();
    }
}
