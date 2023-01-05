use std::time::Duration;

use pollster::FutureExt;
use rumqttc::AsyncClient;
use tokio::task::JoinHandle;
use tracing::{error, debug};

use crate::{config::{MqttDeviceConfig, PresenceDeviceConfig}, mqtt::{OnMqtt, ContactMessage, PresenceMessage}};

use super::Device;

pub struct ContactSensor {
    identifier: String,
    mqtt: MqttDeviceConfig,
    presence: Option<PresenceDeviceConfig>,

    client: AsyncClient,
    is_closed: bool,
    handle: Option<JoinHandle<()>>,
}

impl ContactSensor {
    pub fn new(identifier: String, mqtt: MqttDeviceConfig, presence: Option<PresenceDeviceConfig>, client: AsyncClient) -> Self {
        client.subscribe(mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce).block_on().unwrap();

        Self {
            identifier,
            mqtt,
            presence,
            client,
            is_closed: true,
            handle: None,
        }
    }
}

impl Device for ContactSensor {
    fn get_id(&self) -> String {
        self.identifier.clone()
    }
}

impl OnMqtt for ContactSensor {
    fn on_mqtt(&mut self, message: &rumqttc::Publish) {
        if message.topic != self.mqtt.topic {
            return;
        }

        let is_closed = match ContactMessage::try_from(message) {
            Ok(state) => state.is_closed(),
            Err(err) => {
                error!(id = self.identifier, "Failed to parse message: {err}");
                return;
            },
        };

        if is_closed == self.is_closed {
            return;
        }

        debug!(id = self.identifier, "Updating state to {is_closed}");
        self.is_closed = is_closed;

        // Check if this contact sensor works as a presence device
        // If not we are done here
        let presence = match &self.presence {
            Some(presence) => presence,
            None => return,
        };

        if !is_closed {
            // Activate presence and stop any timeout once we open the door
            if let Some(handle) = self.handle.take() {
                handle.abort();
            }

            self.client.publish(presence.mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce, false, serde_json::to_string(&PresenceMessage::new(true)).unwrap()).block_on().unwrap();
        } else {
            // Once the door is closed again we start a timeout for removing the presence
            let client = self.client.clone();
            let topic = presence.mqtt.topic.clone();
            let id = self.identifier.clone();
            let timeout = Duration::from_secs(presence.timeout);
            self.handle = Some(
                tokio::spawn(async move {
                    debug!(id, "Starting timeout ({timeout:?}) for contact sensor...");
                    tokio::time::sleep(timeout).await;
                    debug!(id, "Removing door device!");
                    client.publish(topic, rumqttc::QoS::AtLeastOnce, false, "").await.unwrap();
                })
            );
        }
    }
}
