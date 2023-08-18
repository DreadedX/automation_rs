use async_trait::async_trait;
use rumqttc::AsyncClient;
use serde::Deserialize;
use tracing::warn;

use crate::device_manager::ConfigExternal;
use crate::device_manager::DeviceConfig;
use crate::devices::Device;
use crate::error::DeviceConfigError;
use crate::event::OnDarkness;
use crate::event::OnPresence;
use crate::{
    config::MqttDeviceConfig,
    messages::{DarknessMessage, PresenceMessage},
};

#[derive(Debug, Deserialize)]
pub struct DebugBridgeConfig {
    #[serde(flatten)]
    pub mqtt: MqttDeviceConfig,
}

#[async_trait]
impl DeviceConfig for DebugBridgeConfig {
    async fn create(
        self,
        identifier: &str,
        ext: &ConfigExternal,
    ) -> Result<Box<dyn Device>, DeviceConfigError> {
        let device = DebugBridge {
            identifier: identifier.into(),
            mqtt: self.mqtt,
            client: ext.client.clone(),
        };

        Ok(Box::new(device))
    }
}

#[derive(Debug)]
pub struct DebugBridge {
    identifier: String,
    mqtt: MqttDeviceConfig,
    client: AsyncClient,
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
        let topic = format!("{}/presence", self.mqtt.topic);
        self.client
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
                    self.mqtt.topic
                )
            })
            .ok();
    }
}

#[async_trait]
impl OnDarkness for DebugBridge {
    async fn on_darkness(&mut self, dark: bool) {
        let message = DarknessMessage::new(dark);
        let topic = format!("{}/darkness", self.mqtt.topic);
        self.client
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
                    self.mqtt.topic
                )
            })
            .ok();
    }
}
