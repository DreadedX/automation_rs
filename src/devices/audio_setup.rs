use async_trait::async_trait;
use automation_macro::LuaDevice;
use google_home::traits::OnOff;
use serde::Deserialize;
use tracing::{debug, error, trace, warn};

use super::Device;
use crate::config::MqttDeviceConfig;
use crate::device_manager::{ConfigExternal, DeviceConfig, WrappedDevice};
use crate::error::DeviceConfigError;
use crate::event::{OnMqtt, OnPresence};
use crate::messages::{RemoteAction, RemoteMessage};

#[derive(Debug, Clone, Deserialize)]
pub struct AudioSetupConfig {
    #[serde(flatten)]
    mqtt: MqttDeviceConfig,
    mixer: String,
    speakers: String,
}

#[async_trait]
impl DeviceConfig for AudioSetupConfig {
    async fn create(
        &self,
        identifier: &str,
        ext: &ConfigExternal,
    ) -> Result<Box<dyn Device>, DeviceConfigError> {
        trace!(id = identifier, "Setting up AudioSetup");

        // TODO: Make sure they implement OnOff?
        let mixer = ext
            .device_manager
            .get(&self.mixer)
            .await
            // NOTE: We need to clone to make the compiler happy, how ever if this clone happens the next one can never happen...
            .ok_or(DeviceConfigError::MissingChild(
                identifier.into(),
                self.mixer.clone(),
            ))?;

        {
            let mixer = mixer.read().await;
            if (mixer.as_ref().cast() as Option<&dyn OnOff>).is_none() {
                return Err(DeviceConfigError::MissingTrait(
                    self.mixer.clone(),
                    "OnOff".into(),
                ));
            }
        }

        let speakers =
            ext.device_manager
                .get(&self.speakers)
                .await
                .ok_or(DeviceConfigError::MissingChild(
                    identifier.into(),
                    self.speakers.clone(),
                ))?;

        {
            let speakers = speakers.read().await;
            if (speakers.as_ref().cast() as Option<&dyn OnOff>).is_none() {
                return Err(DeviceConfigError::MissingTrait(
                    self.mixer.clone(),
                    "OnOff".into(),
                ));
            }
        }

        let device = AudioSetup {
            identifier: identifier.into(),
            config: self.clone(),
            mixer,
            speakers,
        };

        Ok(Box::new(device))
    }
}

// TODO: We need a better way to store the children devices
#[derive(Debug, LuaDevice)]
pub struct AudioSetup {
    identifier: String,
    #[config]
    config: AudioSetupConfig,
    mixer: WrappedDevice,
    speakers: WrappedDevice,
}

impl Device for AudioSetup {
    fn get_id(&self) -> &str {
        &self.identifier
    }
}

#[async_trait]
impl OnMqtt for AudioSetup {
    fn topics(&self) -> Vec<&str> {
        vec![&self.config.mqtt.topic]
    }

    async fn on_mqtt(&mut self, message: rumqttc::Publish) {
        let action = match RemoteMessage::try_from(message) {
            Ok(message) => message.action(),
            Err(err) => {
                error!(id = self.identifier, "Failed to parse message: {err}");
                return;
            }
        };

        let mut mixer = self.mixer.write().await;
        let mut speakers = self.speakers.write().await;
        if let (Some(mixer), Some(speakers)) = (
            mixer.as_mut().cast_mut() as Option<&mut dyn OnOff>,
            speakers.as_mut().cast_mut() as Option<&mut dyn OnOff>,
        ) {
            match action {
				RemoteAction::On => {
					if mixer.is_on().await.unwrap() {
						speakers.set_on(false).await.unwrap();
						mixer.set_on(false).await.unwrap();
					} else {
						speakers.set_on(true).await.unwrap();
						mixer.set_on(true).await.unwrap();
					}
				},
				RemoteAction::BrightnessMoveUp => {
					if !mixer.is_on().await.unwrap() {
						mixer.set_on(true).await.unwrap();
					} else if speakers.is_on().await.unwrap() {
						speakers.set_on(false).await.unwrap();
					} else {
						speakers.set_on(true).await.unwrap();
					}
				},
				RemoteAction::BrightnessStop => { /* Ignore this action */ },
				_ => warn!("Expected ikea shortcut button which only supports 'on' and 'brightness_move_up', got: {action:?}")
			}
        }
    }
}

#[async_trait]
impl OnPresence for AudioSetup {
    async fn on_presence(&mut self, presence: bool) {
        let mut mixer = self.mixer.write().await;
        let mut speakers = self.speakers.write().await;
        if let (Some(mixer), Some(speakers)) = (
            mixer.as_mut().cast_mut() as Option<&mut dyn OnOff>,
            speakers.as_mut().cast_mut() as Option<&mut dyn OnOff>,
        ) {
            // Turn off the audio setup when we leave the house
            if !presence {
                debug!(id = self.identifier, "Turning devices off");
                speakers.set_on(false).await.unwrap();
                mixer.set_on(false).await.unwrap();
            }
        }
    }
}
