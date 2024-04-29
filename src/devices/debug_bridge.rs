use std::convert::Infallible;

use async_trait::async_trait;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use tracing::{trace, warn};

use super::LuaDeviceCreate;
use crate::config::MqttDeviceConfig;
use crate::devices::Device;
use crate::event::{OnDarkness, OnPresence};
use crate::messages::{DarknessMessage, PresenceMessage};
use crate::mqtt::WrappedAsyncClient;

#[derive(Debug, LuaDeviceConfig, Clone)]
pub struct DebugBridgeConfig {
    pub identifier: String,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

#[derive(Debug, LuaDevice)]
pub struct DebugBridge {
    config: DebugBridgeConfig,
}

#[async_trait]
impl LuaDeviceCreate for DebugBridge {
    type Config = DebugBridgeConfig;
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
    async fn on_presence(&mut self, presence: bool) {
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
    async fn on_darkness(&mut self, dark: bool) {
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
