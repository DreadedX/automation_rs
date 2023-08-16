use async_trait::async_trait;
use rumqttc::AsyncClient;
use serde::Deserialize;
use tracing::warn;

use crate::devices::Device;
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

#[derive(Debug)]
pub struct DebugBridge {
    mqtt: MqttDeviceConfig,
    client: AsyncClient,
}

impl DebugBridge {
    pub fn new(config: DebugBridgeConfig, client: &AsyncClient) -> Self {
        Self {
            mqtt: config.mqtt,
            client: client.clone(),
        }
    }
}

impl Device for DebugBridge {
    fn get_id(&self) -> &str {
        "debug_bridge"
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
