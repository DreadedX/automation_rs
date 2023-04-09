use async_trait::async_trait;
use google_home::traits;
use rumqttc::{AsyncClient, matches};
use tracing::{error, warn, debug};

use crate::config::MqttDeviceConfig;
use crate::error::DeviceError;
use crate::mqtt::{OnMqtt, RemoteMessage, RemoteAction};
use crate::presence::OnPresence;

use super::{Device, DeviceBox, AsOnOff};

// TODO: Ideally we store am Arc to the childern devices,
// that way they hook into everything just like all other devices
#[derive(Debug)]
pub struct AudioSetup {
    identifier: String,
    mqtt: MqttDeviceConfig,
    mixer: Box<dyn traits::OnOff + Sync + Send>,
    speakers: Box<dyn traits::OnOff + Sync + Send>,
}

impl AudioSetup {
    pub async fn build(identifier: &str, mqtt: MqttDeviceConfig, mixer: DeviceBox, speakers: DeviceBox, client: AsyncClient) -> Result<Self, DeviceError> {
        // We expect the children devices to implement the OnOff trait
        let mixer_id = mixer.get_id().to_owned();
        let mixer = AsOnOff::consume(mixer)
            .ok_or_else(|| DeviceError::OnOffExpected(mixer_id))?;

        let speakers_id = speakers.get_id().to_owned();
        let speakers = AsOnOff::consume(speakers)
            .ok_or_else(|| DeviceError::OnOffExpected(speakers_id))?;

        client.subscribe(mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce).await?;

        Ok(Self { identifier: identifier.to_owned(), mqtt, mixer, speakers })
    }
}

impl Device for AudioSetup {
    fn get_id(&self) -> &str {
        &self.identifier
    }
}

#[async_trait]
impl OnMqtt for AudioSetup {
    async fn on_mqtt(&mut self, message: &rumqttc::Publish) {
        if !matches(&message.topic, &self.mqtt.topic) {
            return;
        }

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
