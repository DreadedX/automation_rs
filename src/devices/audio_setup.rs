use google_home::traits;
use rumqttc::{AsyncClient, matches};
use tracing::{error, warn};
use pollster::FutureExt as _;

use crate::config::MqttDeviceConfig;
use crate::mqtt::{OnMqtt, RemoteMessage, RemoteAction};

use super::Device;

pub struct AudioSetup {
    identifier: String,
    mqtt: MqttDeviceConfig,
    mixer: Box<dyn traits::OnOff + Sync + Send>,
    speakers: Box<dyn traits::OnOff + Sync + Send>,
}

impl AudioSetup {
    pub fn new(identifier: String, mqtt: MqttDeviceConfig, mixer: Box<dyn traits::OnOff + Sync + Send>, speakers: Box<dyn traits::OnOff + Sync + Send>, client: AsyncClient) -> Self {
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
