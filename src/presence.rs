use std::{sync::Weak, collections::HashMap};

use parking_lot::RwLock;
use tracing::{debug, span, Level, error};
use rumqttc::{AsyncClient, matches};
use pollster::FutureExt as _;

use crate::{mqtt::{OnMqtt, PresenceMessage}, config::MqttDeviceConfig};

pub trait OnPresence {
    fn on_presence(&mut self, presence: bool);
}

pub struct Presence {
    listeners: Vec<Weak<RwLock<dyn OnPresence + Sync + Send>>>,
    devices: HashMap<String, bool>,
    overall_presence: bool,
    mqtt: MqttDeviceConfig,
}

impl Presence {
    pub fn new(mqtt: MqttDeviceConfig, client: AsyncClient) -> Self {
        client.subscribe(mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce).block_on().unwrap();

        Self { listeners: Vec::new(), devices: HashMap::new(), overall_presence: false, mqtt }
    }

    pub fn add_listener<T: OnPresence + Sync + Send + 'static>(&mut self, listener: Weak<RwLock<T>>) {
        self.listeners.push(listener);
    }

    pub fn notify(presence: bool, listeners: Vec<Weak<RwLock<dyn OnPresence + Sync + Send>>>) {
        let _span = span!(Level::TRACE, "presence_update").entered();
        listeners.into_iter().for_each(|listener| {
            if let Some(listener) = listener.upgrade() {
                listener.write().on_presence(presence);
            }
        })
    }
}

impl OnMqtt for Presence {
    fn on_mqtt(&mut self, message: &rumqttc::Publish) {
        if !matches(&message.topic, &self.mqtt.topic) {
            return;
        }

        let offset = self.mqtt.topic.find('+').or(self.mqtt.topic.find('#')).unwrap();
        let device_name = &message.topic[offset..];

        if message.payload.len() == 0 {
            // Remove the device from the map
            debug!("State of device [{device_name}] has been removed");
            self.devices.remove(device_name);
        } else {
            let present = match PresenceMessage::try_from(message) {
                Ok(state) => state.present(),
                Err(err) => {
                    error!("Failed to parse message: {err}");
                    return;
                }
            };

            debug!("State of device [{device_name}] has changed: {}", present);
            self.devices.insert(device_name.to_owned(), present);
        }

        let overall_presence = self.devices.iter().any(|(_, v)| *v);
        if overall_presence != self.overall_presence {
            debug!("Overall presence updated: {overall_presence}");
            self.overall_presence = overall_presence;

            // Remove non-existing listeners
            self.listeners.retain(|listener| listener.strong_count() > 0);
            // Clone the listeners
            let listeners = self.listeners.clone();

            // Notify might block, so we spawn a blocking task
            tokio::task::spawn_blocking(move || {
                Presence::notify(overall_presence, listeners);
            });
        }
    }
}
