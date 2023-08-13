use async_trait::async_trait;
use google_home::traits::OnOff;
use rumqttc::AsyncClient;
use serde::Deserialize;
use tracing::{debug, error, trace, warn};

use crate::{
    config::{CreateDevice, MqttDeviceConfig},
    device_manager::{DeviceManager, WrappedDevice},
    devices::As,
    error::CreateDeviceError,
    event::EventChannel,
    event::OnMqtt,
    event::OnPresence,
    messages::{RemoteAction, RemoteMessage},
};

use super::Device;

#[derive(Debug, Clone, Deserialize)]
pub struct AudioSetupConfig {
    #[serde(flatten)]
    mqtt: MqttDeviceConfig,
    mixer: String,
    speakers: String,
}

// TODO: We need a better way to store the children devices
#[derive(Debug)]
pub struct AudioSetup {
    identifier: String,
    mqtt: MqttDeviceConfig,
    mixer: WrappedDevice,
    speakers: WrappedDevice,
}

#[async_trait]
impl CreateDevice for AudioSetup {
    type Config = AudioSetupConfig;

    async fn create(
        identifier: &str,
        config: Self::Config,
        _event_channel: &EventChannel,
        _client: &AsyncClient,
        _presence_topic: &str,
        device_manager: &DeviceManager,
    ) -> Result<Self, CreateDeviceError> {
        trace!(id = identifier, "Setting up AudioSetup");

        // TODO: Make sure they implement OnOff?
        let mixer = device_manager
            .get(&config.mixer)
            .await
            // NOTE: We need to clone to make the compiler happy, how ever if this clone happens the next one can never happen...
            .ok_or(CreateDeviceError::DeviceDoesNotExist(config.mixer.clone()))?;

        {
            let mixer = mixer.read().await;
            if As::<dyn OnOff>::cast(mixer.as_ref()).is_none() {
                return Err(CreateDeviceError::OnOffExpected(config.mixer));
            }
        }

        let speakers = device_manager.get(&config.speakers).await.ok_or(
            CreateDeviceError::DeviceDoesNotExist(config.speakers.clone()),
        )?;

        {
            let speakers = speakers.read().await;
            if As::<dyn OnOff>::cast(speakers.as_ref()).is_none() {
                return Err(CreateDeviceError::OnOffExpected(config.speakers));
            }
        }

        Ok(Self {
            identifier: identifier.to_owned(),
            mqtt: config.mqtt,
            mixer,
            speakers,
        })
    }
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
