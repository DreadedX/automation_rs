use std::time::Duration;

use async_trait::async_trait;
use google_home::traits::OnOff;
use rumqttc::AsyncClient;
use serde::Deserialize;
use serde_with::{serde_as, DurationSeconds};
use tokio::task::JoinHandle;
use tracing::{debug, error, trace, warn};

use crate::{
    config::MqttDeviceConfig,
    device_manager::{ConfigExternal, DeviceConfig, WrappedDevice},
    devices::{As, DEFAULT_PRESENCE},
    error::DeviceConfigError,
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
    pub mqtt: MqttDeviceConfig,
    #[serde_as(as = "DurationSeconds")]
    pub timeout: Duration,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct TriggerConfig {
    devices: Vec<String>,
    #[serde(default)]
    #[serde_as(as = "DurationSeconds")]
    pub timeout: Duration,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContactSensorConfig {
    #[serde(flatten)]
    mqtt: MqttDeviceConfig,
    presence: Option<PresenceDeviceConfig>,
    trigger: Option<TriggerConfig>,
}

#[async_trait]
impl DeviceConfig for ContactSensorConfig {
    async fn create(
        self,
        identifier: &str,
        ext: &ConfigExternal,
    ) -> Result<Box<dyn Device>, DeviceConfigError> {
        trace!(id = identifier, "Setting up ContactSensor");

        let trigger = if let Some(trigger_config) = &self.trigger {
            let mut devices = Vec::new();
            for device_name in &trigger_config.devices {
                let device = ext.device_manager.get(device_name).await.ok_or(
                    DeviceConfigError::MissingChild(device_name.into(), "OnOff".into()),
                )?;

                if !As::<dyn OnOff>::is(device.read().await.as_ref()) {
                    return Err(DeviceConfigError::MissingTrait(
                        device_name.into(),
                        "OnOff".into(),
                    ));
                }

                if !trigger_config.timeout.is_zero()
                    && !As::<dyn Timeout>::is(device.read().await.as_ref())
                {
                    return Err(DeviceConfigError::MissingTrait(
                        device_name.into(),
                        "Timeout".into(),
                    ));
                }

                devices.push((device, false));
            }

            Some(Trigger {
                devices,
                timeout: trigger_config.timeout,
            })
        } else {
            None
        };

        let device = ContactSensor {
            identifier: identifier.into(),
            mqtt: self.mqtt,
            presence: self.presence,
            client: ext.client.clone(),
            overall_presence: DEFAULT_PRESENCE,
            is_closed: true,
            handle: None,
            trigger,
        };

        Ok(Box::new(device))
    }
}

#[derive(Debug)]
struct Trigger {
    devices: Vec<(WrappedDevice, bool)>,
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

    trigger: Option<Trigger>,
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

        if let Some(trigger) = &mut self.trigger {
            if !self.is_closed {
                for (light, previous) in &mut trigger.devices {
                    let mut light = light.write().await;
                    if let Some(light) = As::<dyn OnOff>::cast_mut(light.as_mut()) {
                        *previous = light.is_on().await.unwrap();
                        light.set_on(true).await.ok();
                    }
                }
            } else {
                for (light, previous) in &trigger.devices {
                    let mut light = light.write().await;
                    if !previous {
                        // If the timeout is zero just turn the light off directly
                        if trigger.timeout.is_zero() && let Some(light) = As::<dyn OnOff>::cast_mut(light.as_mut()) {
                            light.set_on(false).await.ok();
                        } else if let Some(light) = As::<dyn Timeout>::cast_mut(light.as_mut()) {
                            light.start_timeout(trigger.timeout).await.unwrap();
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
                        presence.mqtt.topic.clone(),
                        rumqttc::QoS::AtLeastOnce,
                        false,
                        serde_json::to_string(&PresenceMessage::new(true)).unwrap(),
                    )
                    .await
                    .map_err(|err| {
                        warn!(
                            "Failed to publish presence on {}: {err}",
                            presence.mqtt.topic
                        )
                    })
                    .ok();
            }
        } else {
            // Once the door is closed again we start a timeout for removing the presence
            let client = self.client.clone();
            let id = self.identifier.clone();
            let timeout = presence.timeout;
            let topic = presence.mqtt.topic.clone();
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
