use async_trait::async_trait;
use automation_macro::LuaDeviceConfig;
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
pub struct Config {
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

#[derive(Debug, Clone)]
pub struct AudioSetup {
    config: Config,
}

#[async_trait]
impl LuaDeviceCreate for AudioSetup {
    type Config = Config;
    type Error = DeviceConfigError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.identifier, "Setting up AudioSetup");

        {
            let mixer_id = config.mixer.get_id().to_owned();
            if (config.mixer.cast() as Option<&dyn OnOff>).is_none() {
                return Err(DeviceConfigError::MissingTrait(mixer_id, "OnOff".into()));
            }

            let speakers_id = config.speakers.get_id().to_owned();
            if (config.speakers.cast() as Option<&dyn OnOff>).is_none() {
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
    async fn on_mqtt(&self, message: rumqttc::Publish) {
        if !rumqttc::matches(&message.topic, &self.config.mqtt.topic) {
            return;
        }

        let action = match RemoteMessage::try_from(message) {
            Ok(message) => message.action(),
            Err(err) => {
                error!(id = self.get_id(), "Failed to parse message: {err}");
                return;
            }
        };

        if let (Some(mixer), Some(speakers)) = (
            self.config.mixer.cast() as Option<&dyn OnOff>,
            self.config.speakers.cast() as Option<&dyn OnOff>,
        ) {
            match action {
				RemoteAction::On => {
					if mixer.on().await.unwrap() {
						speakers.set_on(false).await.unwrap();
						mixer.set_on(false).await.unwrap();
					} else {
						speakers.set_on(true).await.unwrap();
						mixer.set_on(true).await.unwrap();
					}
				},
				RemoteAction::BrightnessMoveUp => {
					if !mixer.on().await.unwrap() {
						mixer.set_on(true).await.unwrap();
					} else if speakers.on().await.unwrap() {
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
    async fn on_presence(&self, presence: bool) {
        if let (Some(mixer), Some(speakers)) = (
            self.config.mixer.cast() as Option<&dyn OnOff>,
            self.config.speakers.cast() as Option<&dyn OnOff>,
        ) {
            // Turn off the audio setup when we leave the house
            if !presence {
                debug!(id = self.get_id(), "Turning devices off");
                speakers.set_on(false).await.unwrap();
                mixer.set_on(false).await.unwrap();
            }
        }
    }
}
