use rumqttc::{AsyncClient, matches};
use tracing::{error, warn};
use pollster::FutureExt as _;

use crate::config::MqttDeviceConfig;
use crate::devices::AsOnOff;
use crate::mqtt::{OnMqtt, RemoteMessage, RemoteAction};

use super::{Device, DeviceBox};

pub struct AudioSetup {
    identifier: String,
    mqtt: MqttDeviceConfig,
    mixer: DeviceBox,
    speakers: DeviceBox,
}

impl AudioSetup {
    pub fn new(identifier: String, mqtt: MqttDeviceConfig, mixer: DeviceBox, speakers: DeviceBox, client: AsyncClient) -> Self {
        client.subscribe(mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce).block_on().unwrap();

        Self { identifier, mqtt, mixer, speakers }
    }
}

impl Device for AudioSetup {
    fn get_id(&self) -> String {
        self.identifier.clone()
    }
}

impl OnMqtt for AudioSetup {
    fn on_mqtt(&mut self, message: &rumqttc::Publish) {
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

        let mixer = match AsOnOff::cast_mut(self.mixer.as_mut()) {
            Some(mixer) => mixer,
            None => {
                error!(id = self.identifier, "Mixer device '{}' does not implement OnOff trait", self.mixer.get_id());
                return;
            },
        };
        let speakers = match AsOnOff::cast_mut(self.speakers.as_mut()) {
            Some(speakers) => speakers,
            None => {
                error!(id = self.identifier, "Speakers device '{}' does not implement OnOff trait", self.mixer.get_id());
                return;
            },
        };

        match action {
            RemoteAction::On => {
                if mixer.is_on().unwrap() {
                    speakers.set_on(false).unwrap();
                    mixer.set_on(false).unwrap();
                } else {
                    speakers.set_on(true).unwrap();
                    mixer.set_on(true).unwrap();
                }
            },
            RemoteAction::BrightnessMoveUp => {
                if !mixer.is_on().unwrap() {
                    mixer.set_on(true).unwrap();
                } else if speakers.is_on().unwrap() {
                    speakers.set_on(false).unwrap();
                } else {
                    speakers.set_on(true).unwrap();
                }
            },
            RemoteAction::BrightnessStop => { /* Ignore this action */ },
            _ => warn!("Expected ikea shortcut button which only supports 'on' and 'brightness_move_up', got: {action:?}")
        }
    }
}
