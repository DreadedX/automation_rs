use std::time::Duration;

use async_trait::async_trait;
use rumqttc::{has_wildcards, AsyncClient};
use serde::Deserialize;
use tokio::task::JoinHandle;
use tracing::{debug, error, trace, warn};

use crate::{
    config::{CreateDevice, MqttDeviceConfig},
    error::{CreateDeviceError, MissingWildcard},
    mqtt::{ContactMessage, OnMqtt, PresenceMessage},
    presence::OnPresence,
};

use super::Device;

// NOTE: If we add more presence devices we might need to move this out of here
#[derive(Debug, Clone, Deserialize)]
pub struct PresenceDeviceConfig {
    #[serde(flatten)]
    pub mqtt: Option<MqttDeviceConfig>,
    pub timeout: u64, // Timeout in seconds
}

impl PresenceDeviceConfig {
    /// Set the mqtt topic to an appropriate value if it is not already set
    fn generate_topic(
        mut self,
        class: &str,
        identifier: &str,
        presence_topic: &str,
    ) -> Result<PresenceDeviceConfig, MissingWildcard> {
        if self.mqtt.is_none() {
            if !has_wildcards(presence_topic) {
                return Err(MissingWildcard::new(presence_topic));
            }

            // TODO: This is not perfect, if the topic is some/+/thing/# this will fail
            let offset = presence_topic
                .find('+')
                .or(presence_topic.find('#'))
                .unwrap();
            let topic = format!("{}/{class}/{identifier}", &presence_topic[..offset - 1]);
            trace!("Setting presence mqtt topic: {topic}");
            self.mqtt = Some(MqttDeviceConfig { topic });
        }

        Ok(self)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContactSensorConfig {
    #[serde(flatten)]
    mqtt: MqttDeviceConfig,
    presence: Option<PresenceDeviceConfig>,
}

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

impl CreateDevice for ContactSensor {
    type Config = ContactSensorConfig;

    fn create(
        identifier: &str,
        config: Self::Config,
        client: AsyncClient,
        presence_topic: &str,
    ) -> Result<Self, CreateDeviceError> {
        trace!(id = identifier, "Setting up ContactSensor");

        let presence = config
            .presence
            .map(|p| p.generate_topic("contact", identifier, presence_topic))
            .transpose()?;

        Ok(Self {
            identifier: identifier.to_owned(),
            mqtt: config.mqtt,
            presence,
            client,
            overall_presence: false,
            is_closed: true,
            handle: None,
        })
    }
}

impl Device for ContactSensor {
    fn get_id(&self) -> &str {
        &self.identifier
    }
}

#[async_trait]
impl OnPresence for ContactSensor {
    async fn on_presence(&mut self, presence: bool) {
        self.overall_presence = presence;
    }
}

#[async_trait]
impl OnMqtt for ContactSensor {
    fn topics(&self) -> Vec<&str> {
        vec![&self.mqtt.topic]
    }

    async fn on_mqtt(&mut self, message: &rumqttc::Publish) {
        let is_closed = match ContactMessage::try_from(message) {
            Ok(state) => state.is_closed(),
            Err(err) => {
                error!(id = self.identifier, "Failed to parse message: {err}");
                return;
            }
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
                self.client
                    .publish(
                        topic.clone(),
                        rumqttc::QoS::AtLeastOnce,
                        false,
                        serde_json::to_string(&PresenceMessage::new(true)).unwrap(),
                    )
                    .await
                    .map_err(|err| warn!("Failed to publish presence on {topic}: {err}"))
                    .ok();
            }
        } else {
            // Once the door is closed again we start a timeout for removing the presence
            let client = self.client.clone();
            let id = self.identifier.clone();
            let timeout = Duration::from_secs(presence.timeout);
            self.handle = Some(tokio::spawn(async move {
                debug!(id, "Starting timeout ({timeout:?}) for contact sensor...");
                tokio::time::sleep(timeout).await;
                debug!(id, "Removing door device!");
                client
                    .publish(topic.clone(), rumqttc::QoS::AtLeastOnce, false, "")
                    .await
                    .map_err(|err| warn!("Failed to publish presence on {topic}: {err}"))
                    .ok();
            }));
        }
    }
}
