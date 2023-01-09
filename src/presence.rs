use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::watch;
use tracing::{debug, error};
use rumqttc::{AsyncClient, matches};
use pollster::FutureExt as _;

use crate::{mqtt::{OnMqtt, PresenceMessage, self}, config::MqttDeviceConfig};

#[async_trait]
pub trait OnPresence {
    async fn on_presence(&mut self, presence: bool);
}

pub type Receiver = watch::Receiver<bool>;
type Sender = watch::Sender<bool>;

struct Presence {
    devices: HashMap<String, bool>,
    overall_presence: Receiver,
    mqtt: MqttDeviceConfig,
    tx: Sender,
}

pub fn start(mut mqtt_rx: mqtt::Receiver, mqtt: MqttDeviceConfig, client: AsyncClient) -> Receiver {
    // Subscribe to the relevant topics on mqtt
    client.subscribe(mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce).block_on().unwrap();

    let (tx, overall_presence) = watch::channel(false);
    let mut presence = Presence { devices: HashMap::new(), overall_presence: overall_presence.clone(), mqtt, tx };

    tokio::spawn(async move {
        while mqtt_rx.changed().await.is_ok() {
            if let Some(message) = &*mqtt_rx.borrow() {
                presence.on_mqtt(message);
            }
        }

        unreachable!("Did not expect this");
    });

    return overall_presence;
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
        if overall_presence != *self.overall_presence.borrow() {
            debug!("Overall presence updated: {overall_presence}");
            self.tx.send(overall_presence).ok();
        }
    }
}
