use std::collections::HashMap;

use async_trait::async_trait;
use rumqttc::Publish;
use serde::Deserialize;
use tracing::{debug, warn};

use crate::{
    config::MqttDeviceConfig,
    devices::Device,
    event::{self, Event, EventChannel},
    messages::PresenceMessage,
    traits::OnMqtt,
};

#[derive(Debug, Deserialize)]
pub struct PresenceConfig {
    #[serde(flatten)]
    pub mqtt: MqttDeviceConfig,
}

pub const DEFAULT: bool = false;

#[derive(Debug)]
pub struct Presence {
    tx: event::Sender,
    mqtt: MqttDeviceConfig,
    devices: HashMap<String, bool>,
    current_overall_presence: bool,
}

impl Presence {
    pub fn new(config: PresenceConfig, event_channel: &EventChannel) -> Self {
        Self {
            tx: event_channel.get_tx(),
            mqtt: config.mqtt,
            devices: HashMap::new(),
            current_overall_presence: DEFAULT,
        }
    }
}

impl Device for Presence {
    fn get_id(&self) -> &str {
        "presence"
    }
}

#[async_trait]
impl OnMqtt for Presence {
    fn topics(&self) -> Vec<&str> {
        vec![&self.mqtt.topic]
    }

    async fn on_mqtt(&mut self, message: Publish) {
        let offset = self
            .mqtt
            .topic
            .find('+')
            .or(self.mqtt.topic.find('#'))
            .expect("Presence::create fails if it does not contain wildcards");
        let device_name = message.topic[offset..].to_owned();

        if message.payload.is_empty() {
            // Remove the device from the map
            debug!("State of device [{device_name}] has been removed");
            self.devices.remove(&device_name);
        } else {
            let present = match PresenceMessage::try_from(message) {
                Ok(state) => state.presence(),
                Err(err) => {
                    warn!("Failed to parse message: {err}");
                    return;
                }
            };

            debug!("State of device [{device_name}] has changed: {}", present);
            self.devices.insert(device_name, present);
        }

        let overall_presence = self.devices.iter().any(|(_, v)| *v);
        if overall_presence != self.current_overall_presence {
            debug!("Overall presence updated: {overall_presence}");
            self.current_overall_presence = overall_presence;

            if self
                .tx
                .send(Event::Presence(overall_presence))
                .await
                .is_err()
            {
                warn!("There are no receivers on the event channel");
            }
        }
    }
}
