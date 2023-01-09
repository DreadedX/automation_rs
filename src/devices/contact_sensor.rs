use std::time::Duration;

use async_trait::async_trait;
use pollster::FutureExt;
use rumqttc::{AsyncClient, matches};
use tokio::task::JoinHandle;
use tracing::{error, debug, warn};

use crate::{config::{MqttDeviceConfig, PresenceDeviceConfig}, mqtt::{OnMqtt, ContactMessage, PresenceMessage}, presence::OnPresence};

use super::Device;

#[derive(Debug)]
pub struct ContactSensor {
    identifier: String,
    mqtt: MqttDeviceConfig,
    presence: Option<PresenceDeviceConfig>,

    client: AsyncClient,
    overall_presence: bool,
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
            overall_presence: false,
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

#[async_trait]
impl OnPresence for ContactSensor {
    async fn on_presence(&mut self, presence: bool) {
        self.overall_presence = presence;
    }
}

impl OnMqtt for ContactSensor {
    fn on_mqtt(&mut self, message: &rumqttc::Publish) {
        if !matches(&message.topic, &self.mqtt.topic) {
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

        let topic = match &presence.mqtt {
            Some(mqtt) => mqtt.topic.clone(),
            None => {
                warn!("Contact sensors is configured as a presence sensor, but no mqtt topic is specified");
                return;
            }
        };

        if !is_closed {
            // Activate presence and stop any timeout once we open the door
            if let Some(handle) = self.handle.take() {
                handle.abort();
            }

            // Only use the door as an presence sensor if there the current presence is set false
            // This is to prevent the house from being marked as present for however long the
            // timeout is set when leaving the house
            if !self.overall_presence {
                self.client.publish(topic, rumqttc::QoS::AtLeastOnce, false, serde_json::to_string(&PresenceMessage::new(true)).unwrap()).block_on().unwrap();
            }
        } else {
            // Once the door is closed again we start a timeout for removing the presence
            let client = self.client.clone();
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
