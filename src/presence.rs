use std::{sync::{Weak, RwLock}, collections::HashMap};

use log::{debug, warn, trace};
use rumqttc::{AsyncClient, Publish};
use serde::{Serialize, Deserialize};

use crate::{mqtt::OnMqtt, config::MqttDeviceConfig};

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
        // @TODO Handle potential errors here
        let topic = mqtt.topic.clone() + "/+";
        tokio::spawn(async move {
            client.subscribe(topic, rumqttc::QoS::AtLeastOnce).await.unwrap();
        });

        Self { listeners: Vec::new(), devices: HashMap::new(), overall_presence: false, mqtt }
    }

    pub fn add_listener<T: OnPresence + Sync + Send + 'static>(&mut self, listener: Weak<RwLock<T>>) {
        self.listeners.push(listener);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StateMessage {
    state: bool
}

impl TryFrom<&Publish> for StateMessage {
    type Error = anyhow::Error;

    fn try_from(message: &Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(anyhow::anyhow!("Invalid message payload received: {:?}", message.payload)))
    }
}

impl OnMqtt for Presence {
    fn on_mqtt(&mut self, message: &rumqttc::Publish) {
        if message.topic.starts_with(&(self.mqtt.topic.clone() + "/")) {
            let device_name = message.topic.rsplit_once("/").unwrap().1;

            if message.payload.len() == 0 {
                // Remove the device from the map
                debug!("State of device [{device_name}] has been removed");
                self.devices.remove(device_name);
                return;
            } else {
                let state = match StateMessage::try_from(message) {
                    Ok(state) => state,
                    Err(err) => {
                        warn!("Failed to parse message: {err}");
                        return;
                    }
                };

                debug!("State of device [{device_name}] has changed: {}", state.state);
                self.devices.insert(device_name.to_owned(), state.state);
            }

            let overall_presence = self.devices.iter().any(|(_, v)| *v);
            if overall_presence != self.overall_presence {
                debug!("Overall presence updated: {overall_presence}");
                self.overall_presence = overall_presence;

                trace!("Listener count: {}", self.listeners.len());

                self.listeners.retain(|listener| {
                    if let Some(listener) = listener.upgrade() {
                        listener.write().unwrap().on_presence(overall_presence);
                        return true;
                    } else {
                        trace!("Removing listener...");
                    }

                    return false;
                })
            }
        }
    }
}
