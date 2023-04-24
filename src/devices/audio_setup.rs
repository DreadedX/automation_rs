use async_trait::async_trait;
use google_home::traits;
use rumqttc::AsyncClient;
use serde::Deserialize;
use tracing::{debug, error, trace, warn};

use crate::{
    config::{self, CreateDevice, MqttDeviceConfig},
    error::CreateDeviceError,
    event::EventChannel,
    event::OnMqtt,
    event::OnPresence,
    messages::{RemoteAction, RemoteMessage},
};

use super::{As, Device};

#[derive(Debug, Clone, Deserialize)]
pub struct AudioSetupConfig {
    #[serde(flatten)]
    mqtt: MqttDeviceConfig,
    mixer: Box<config::DeviceConfig>,
    speakers: Box<config::DeviceConfig>,
}

// TODO: We need a better way to store the children devices
#[derive(Debug)]
pub struct AudioSetup {
    identifier: String,
    mqtt: MqttDeviceConfig,
    mixer: Box<dyn traits::OnOff>,
    speakers: Box<dyn traits::OnOff>,
}

impl CreateDevice for AudioSetup {
    type Config = AudioSetupConfig;

    fn create(
        identifier: &str,
        config: Self::Config,
        event_channel: &EventChannel,
        client: &AsyncClient,
        presence_topic: &str,
    ) -> Result<Self, CreateDeviceError> {
        trace!(id = identifier, "Setting up AudioSetup");

        // Create the child devices
        let mixer_id = format!("{}.mixer", identifier);
        let mixer = (*config.mixer).create(&mixer_id, event_channel, client, presence_topic)?;
        let mixer = As::consume(mixer).ok_or(CreateDeviceError::OnOffExpected(mixer_id))?;

        let speakers_id = format!("{}.speakers", identifier);
        let speakers =
            (*config.speakers).create(&speakers_id, event_channel, client, presence_topic)?;
        let speakers =
            As::consume(speakers).ok_or(CreateDeviceError::OnOffExpected(speakers_id))?;

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

        match action {
            RemoteAction::On => {
                if self.mixer.is_on().unwrap() {
                    self.speakers.set_on(false).unwrap();
                    self.mixer.set_on(false).unwrap();
                } else {
                    self.speakers.set_on(true).unwrap();
                    self.mixer.set_on(true).unwrap();
                }
            },
            RemoteAction::BrightnessMoveUp => {
                if !self.mixer.is_on().unwrap() {
                    self.mixer.set_on(true).unwrap();
                } else if self.speakers.is_on().unwrap() {
                    self.speakers.set_on(false).unwrap();
                } else {
                    self.speakers.set_on(true).unwrap();
                }
            },
            RemoteAction::BrightnessStop => { /* Ignore this action */ },
            _ => warn!("Expected ikea shortcut button which only supports 'on' and 'brightness_move_up', got: {action:?}")
        }
    }
}

#[async_trait]
impl OnPresence for AudioSetup {
    async fn on_presence(&mut self, presence: bool) {
        // Turn off the audio setup when we leave the house
        if !presence {
            debug!(id = self.identifier, "Turning devices off");
            self.speakers.set_on(false).unwrap();
            self.mixer.set_on(false).unwrap();
        }
    }
}
