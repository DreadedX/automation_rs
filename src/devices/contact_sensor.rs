use std::time::Duration;

use async_trait::async_trait;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use google_home::traits::OnOff;
use mlua::FromLua;
use tokio::task::JoinHandle;
use tracing::{debug, error, trace, warn};

use super::Device;
use crate::config::MqttDeviceConfig;
use crate::device_manager::{DeviceConfig, WrappedDevice};
use crate::devices::DEFAULT_PRESENCE;
use crate::error::DeviceConfigError;
use crate::event::{OnMqtt, OnPresence};
use crate::messages::{ContactMessage, PresenceMessage};
use crate::mqtt::WrappedAsyncClient;
use crate::traits::Timeout;

// NOTE: If we add more presence devices we might need to move this out of here
#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct PresenceDeviceConfig {
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(with(Duration::from_secs))]
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
struct TriggerDevicesHelper(Vec<WrappedDevice>);

impl<'lua> FromLua<'lua> for TriggerDevicesHelper {
    fn from_lua(value: mlua::Value<'lua>, lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        Ok(TriggerDevicesHelper(mlua::FromLua::from_lua(value, lua)?))
    }
}

impl From<TriggerDevicesHelper> for Vec<(WrappedDevice, bool)> {
    fn from(value: TriggerDevicesHelper) -> Self {
        value.0.into_iter().map(|device| (device, false)).collect()
    }
}

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct TriggerConfig {
    #[device_config(from_lua, from(TriggerDevicesHelper))]
    devices: Vec<(WrappedDevice, bool)>,
    #[device_config(default, with(|t: Option<_>| t.map(Duration::from_secs)))]
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct ContactSensorConfig {
    #[device_config(flatten)]
    mqtt: MqttDeviceConfig,
    #[device_config(from_lua)]
    presence: Option<PresenceDeviceConfig>,
    #[device_config(from_lua)]
    trigger: Option<TriggerConfig>,
    #[device_config(from_lua)]
    client: WrappedAsyncClient,
}

#[async_trait]
impl DeviceConfig for ContactSensorConfig {
    async fn create(&self, identifier: &str) -> Result<Box<dyn Device>, DeviceConfigError> {
        trace!(id = identifier, "Setting up ContactSensor");

        // Make sure the devices implement the required traits
        if let Some(trigger) = &self.trigger {
            for (device, _) in &trigger.devices {
                {
                    let device = device.read().await;
                    let id = device.get_id().to_owned();
                    if (device.as_ref().cast() as Option<&dyn OnOff>).is_none() {
                        return Err(DeviceConfigError::MissingTrait(id, "OnOff".into()));
                    }

                    if trigger.timeout.is_none()
                        && (device.as_ref().cast() as Option<&dyn Timeout>).is_none()
                    {
                        return Err(DeviceConfigError::MissingTrait(id, "Timeout".into()));
                    }
                }
            }
        }

        let device = ContactSensor {
            identifier: identifier.into(),
            config: self.clone(),
            overall_presence: DEFAULT_PRESENCE,
            is_closed: true,
            handle: None,
        };

        Ok(Box::new(device))
    }
}

#[derive(Debug, LuaDevice)]
pub struct ContactSensor {
    identifier: String,
    #[config]
    config: ContactSensorConfig,

    overall_presence: bool,
    is_closed: bool,
    handle: Option<JoinHandle<()>>,
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
        vec![&self.config.mqtt.topic]
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

        if let Some(trigger) = &mut self.config.trigger {
            if !self.is_closed {
                for (light, previous) in &mut trigger.devices {
                    let mut light = light.write().await;
                    if let Some(light) = light.as_mut().cast_mut() as Option<&mut dyn OnOff> {
                        *previous = light.is_on().await.unwrap();
                        light.set_on(true).await.ok();
                    }
                }
            } else {
                for (light, previous) in &trigger.devices {
                    let mut light = light.write().await;
                    if !previous {
                        // If the timeout is zero just turn the light off directly
                        if trigger.timeout.is_none()
                            && let Some(light) = light.as_mut().cast_mut() as Option<&mut dyn OnOff>
                        {
                            light.set_on(false).await.ok();
                        } else if let Some(timeout) = trigger.timeout
                            && let Some(light) =
                                light.as_mut().cast_mut() as Option<&mut dyn Timeout>
                        {
                            light.start_timeout(timeout).await.unwrap();
                        }
                        // TODO: Put a warning/error on creation if either of this has to option to fail
                    }
                }
            }
        }

        // Check if this contact sensor works as a presence device
        // If not we are done here
        let presence = match &self.config.presence {
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
                self.config
                    .client
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
            let client = self.config.client.clone();
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
