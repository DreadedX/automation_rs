use async_trait::async_trait;
use rumqttc::AsyncClient;
use tracing::warn;

use crate::{config::DebugBridgeConfig, presence::{OnPresence, self}, light_sensor::{OnDarkness, self}, mqtt::{PresenceMessage, DarknessMessage}};

struct DebugBridge {
    topic: String,
    client: AsyncClient,
}

impl DebugBridge {
    pub fn new(topic: &str, client: AsyncClient) -> Self {
        Self { topic: topic.to_owned(), client }
    }
}

pub fn start(mut presence_rx: presence::Receiver, mut light_sensor_rx: light_sensor::Receiver, config: &DebugBridgeConfig, client: AsyncClient) {
    let mut debug_bridge = DebugBridge::new(&config.topic, client);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                res = presence_rx.changed() => {
                    if !res.is_ok() {
                        break;
                    }

                    let presence = *presence_rx.borrow();
                    debug_bridge.on_presence(presence).await;
                }
                res = light_sensor_rx.changed() => {
                    if !res.is_ok() {
                        break;
                    }

                    let darkness = *light_sensor_rx.borrow();
                    debug_bridge.on_darkness(darkness).await;
                }
            }
        }

        unreachable!("Did not expect this");
    });
}

#[async_trait]
impl OnPresence for DebugBridge {
    async fn on_presence(&mut self, presence: bool) {
        let message = PresenceMessage::new(presence);
        let topic = format!("{}/presence", self.topic);
        self.client.publish(topic, rumqttc::QoS::AtLeastOnce, true, serde_json::to_string(&message).unwrap())
            .await
            .map_err(|err| warn!("Failed to update presence on {}/presence: {err}", self.topic))
            .ok();
    }
}

#[async_trait]
impl OnDarkness for DebugBridge {
    async fn on_darkness(&mut self, dark: bool) {
        let message = DarknessMessage::new(dark);
        let topic = format!("{}/darkness", self.topic);
        self.client.publish(topic, rumqttc::QoS::AtLeastOnce, true, serde_json::to_string(&message).unwrap())
            .await
            .map_err(|err| warn!("Failed to update presence on {}/presence: {err}", self.topic))
            .ok();
    }
}
