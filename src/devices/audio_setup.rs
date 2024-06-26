use async_trait::async_trait;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use google_home::traits::OnOff;
use tracing::{debug, error, trace, warn};

use super::{Device, LuaDeviceCreate};
use crate::config::MqttDeviceConfig;
use crate::device_manager::WrappedDevice;
use crate::error::DeviceConfigError;
use crate::event::{OnMqtt, OnPresence};
use crate::messages::{RemoteAction, RemoteMessage};
use crate::mqtt::WrappedAsyncClient;

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct AudioSetupConfig {
    pub identifier: String,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(from_lua)]
    pub mixer: WrappedDevice,
    #[device_config(from_lua)]
    pub speakers: WrappedDevice,
    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

#[derive(Debug, LuaDevice)]
pub struct AudioSetup {
    config: AudioSetupConfig,
}

#[async_trait]
impl LuaDeviceCreate for AudioSetup {
    type Config = AudioSetupConfig;
    type Error = DeviceConfigError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.identifier, "Setting up AudioSetup");

        {
            let mixer = config.mixer.read().await;
            let mixer_id = mixer.get_id().to_owned();
            if (mixer.as_ref().cast() as Option<&dyn OnOff>).is_none() {
                return Err(DeviceConfigError::MissingTrait(mixer_id, "OnOff".into()));
            }

            let speakers = config.speakers.read().await;
            let speakers_id = speakers.get_id().to_owned();
            if (speakers.as_ref().cast() as Option<&dyn OnOff>).is_none() {
                return Err(DeviceConfigError::MissingTrait(speakers_id, "OnOff".into()));
            }
        }

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        Ok(AudioSetup { config })
    }
}

impl Device for AudioSetup {
    fn get_id(&self) -> String {
        self.config.identifier.clone()
    }
}

#[async_trait]
impl OnMqtt for AudioSetup {
    async fn on_mqtt(&mut self, message: rumqttc::Publish) {
        if !rumqttc::matches(&message.topic, &self.config.mqtt.topic) {
            return;
        }

        let action = match RemoteMessage::try_from(message) {
            Ok(message) => message.action(),
            Err(err) => {
                error!(
                    id = self.config.identifier,
                    "Failed to parse message: {err}"
                );
                return;
            }
        };

        let mut mixer = self.config.mixer.write().await;
        let mut speakers = self.config.speakers.write().await;
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
        let mut mixer = self.config.mixer.write().await;
        let mut speakers = self.config.speakers.write().await;

        if let (Some(mixer), Some(speakers)) = (
            mixer.as_mut().cast_mut() as Option<&mut dyn OnOff>,
            speakers.as_mut().cast_mut() as Option<&mut dyn OnOff>,
        ) {
            // Turn off the audio setup when we leave the house
            if !presence {
                debug!(id = self.config.identifier, "Turning devices off");
                speakers.set_on(false).await.unwrap();
                mixer.set_on(false).await.unwrap();
            }
        }
    }
}
