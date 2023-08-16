use std::time::Duration;

use async_trait::async_trait;
use google_home::traits::OnOff;
use rumqttc::{has_wildcards, AsyncClient};
use serde::Deserialize;
use serde_with::{serde_as, DurationSeconds};
use tokio::task::JoinHandle;
use tracing::{debug, error, trace, warn};

use crate::{
    config::{ConfigExternal, DeviceConfig, MqttDeviceConfig},
    device_manager::WrappedDevice,
    devices::{As, DEFAULT_PRESENCE},
    error::{DeviceConfigError, MissingWildcard},
    event::OnMqtt,
    event::OnPresence,
    messages::{ContactMessage, PresenceMessage},
    traits::Timeout,
};

use super::Device;

// NOTE: If we add more presence devices we might need to move this out of here
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct PresenceDeviceConfig {
    #[serde(flatten)]
    pub mqtt: Option<MqttDeviceConfig>,
    #[serde_as(as = "DurationSeconds")]
    pub timeout: Duration,
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

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct LightsConfig {
    lights: Vec<String>,
    #[serde(default)]
    #[serde_as(as = "DurationSeconds")]
    pub timeout: Duration,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContactSensorConfig {
    #[serde(flatten)]
    mqtt: MqttDeviceConfig,
    presence: Option<PresenceDeviceConfig>,
    lights: Option<LightsConfig>,
}

#[async_trait]
impl DeviceConfig for ContactSensorConfig {
    async fn create(
        self,
        identifier: &str,
        ext: &ConfigExternal,
    ) -> Result<Box<dyn Device>, DeviceConfigError> {
        trace!(id = identifier, "Setting up ContactSensor");

        let presence = self
            .presence
            .map(|p| p.generate_topic("contact", identifier, ext.presence_topic))
            .transpose()?;

        let lights =
            if let Some(lights_config) = self.lights {
                let mut lights = Vec::new();
                for name in lights_config.lights {
                    let light = ext.device_manager.get(&name).await.ok_or(
                        DeviceConfigError::MissingChild(name.clone(), "OnOff".into()),
                    )?;

                    if !As::<dyn OnOff>::is(light.read().await.as_ref()) {
                        return Err(DeviceConfigError::MissingTrait(name, "OnOff".into()));
                    }

                    lights.push((light, false));
                }

                Some(Lights {
                    lights,
                    timeout: lights_config.timeout,
                })
            } else {
                None
            };

        let device = ContactSensor {
            identifier: identifier.to_owned(),
            mqtt: self.mqtt,
            presence,
            client: ext.client.clone(),
            overall_presence: DEFAULT_PRESENCE,
            is_closed: true,
            handle: None,
            lights,
        };

        Ok(Box::new(device))
    }
}

#[derive(Debug)]
struct Lights {
    lights: Vec<(WrappedDevice, bool)>,
    timeout: Duration, // Timeout in seconds
}

#[derive(Debug)]
struct ContactSensor {
    identifier: String,
    mqtt: MqttDeviceConfig,
    presence: Option<PresenceDeviceConfig>,

    client: AsyncClient,
    overall_presence: bool,
    is_closed: bool,
    handle: Option<JoinHandle<()>>,

    lights: Option<Lights>,
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

    async fn on_mqtt(&mut self, message: rumqttc::Publish) {
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

        if let Some(lights) = &mut self.lights {
            if !self.is_closed {
                for (light, previous) in &mut lights.lights {
                    let mut light = light.write().await;
                    if let Some(light) = As::<dyn OnOff>::cast_mut(light.as_mut()) {
                        *previous = light.is_on().await.unwrap();
                        light.set_on(true).await.ok();
                    }
                }
            } else {
                for (light, previous) in &lights.lights {
                    let mut light = light.write().await;
                    if !previous {
                        // If the timeout is zero just turn the light off directly
                        if lights.timeout.is_zero() && let Some(light) = As::<dyn OnOff>::cast_mut(light.as_mut()) {
                            light.set_on(false).await.ok();
                        } else if let Some(light) = As::<dyn Timeout>::cast_mut(light.as_mut()) {
                            light.start_timeout(lights.timeout).await;
                        }
                        // TODO: Put a warning/error on creation if either of this has to option to fail
                    }
                }
            }
        }

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
            let timeout = presence.timeout;
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
