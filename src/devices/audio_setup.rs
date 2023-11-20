use async_trait::async_trait;
use google_home::traits::OnOff;
use serde::Deserialize;
use tracing::{debug, error, trace, warn};

use super::Device;
use crate::config::MqttDeviceConfig;
use crate::device_manager::{ConfigExternal, DeviceConfig, WrappedDevice};
use crate::devices::As;
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
        self,
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

        if !As::<dyn OnOff>::is(mixer.read().await.as_ref()) {
            return Err(DeviceConfigError::MissingTrait(self.mixer, "OnOff".into()));
        }

        let speakers =
            ext.device_manager
                .get(&self.speakers)
                .await
                .ok_or(DeviceConfigError::MissingChild(
                    identifier.into(),
                    self.speakers.clone(),
                ))?;

        if !As::<dyn OnOff>::is(speakers.read().await.as_ref()) {
            return Err(DeviceConfigError::MissingTrait(
                self.speakers,
                "OnOff".into(),
            ));
        }

        let device = AudioSetup {
            identifier: identifier.into(),
            mqtt: self.mqtt,
            mixer,
            speakers,
        };

        Ok(Box::new(device))
    }
}

// TODO: We need a better way to store the children devices
#[derive(Debug)]
struct AudioSetup {
    identifier: String,
    mqtt: MqttDeviceConfig,
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
        vec![&self.mqtt.topic]
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
            As::<dyn OnOff>::cast_mut(mixer.as_mut()),
            As::<dyn OnOff>::cast_mut(speakers.as_mut()),
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
            As::<dyn OnOff>::cast_mut(mixer.as_mut()),
            As::<dyn OnOff>::cast_mut(speakers.as_mut()),
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
