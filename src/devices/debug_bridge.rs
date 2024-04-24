use async_trait::async_trait;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use tracing::warn;

use crate::config::MqttDeviceConfig;
use crate::device_manager::DeviceConfig;
use crate::devices::Device;
use crate::error::DeviceConfigError;
use crate::event::{OnDarkness, OnPresence};
use crate::messages::{DarknessMessage, PresenceMessage};
use crate::mqtt::WrappedAsyncClient;

#[derive(Debug, LuaDeviceConfig, Clone)]
pub struct DebugBridgeConfig {
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(user_data)]
    client: WrappedAsyncClient,
}

#[async_trait]
impl DeviceConfig for DebugBridgeConfig {
    async fn create(&self, identifier: &str) -> Result<Box<dyn Device>, DeviceConfigError> {
        let device = DebugBridge {
            identifier: identifier.into(),
            config: self.clone(),
        };

        Ok(Box::new(device))
    }
}

#[derive(Debug, LuaDevice)]
pub struct DebugBridge {
    identifier: String,
    #[config]
    config: DebugBridgeConfig,
}

impl Device for DebugBridge {
    fn get_id(&self) -> &str {
        &self.identifier
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
