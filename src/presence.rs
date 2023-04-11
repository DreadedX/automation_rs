use std::collections::HashMap;

use async_trait::async_trait;
use rumqttc::{has_wildcards, matches, AsyncClient};
use tokio::sync::watch;
use tracing::{debug, error};

use crate::{
    config::MqttDeviceConfig,
    error::{MissingWildcard, PresenceError},
    mqtt::{self, OnMqtt, PresenceMessage},
};

#[async_trait]
pub trait OnPresence: Sync + Send + 'static {
    async fn on_presence(&mut self, presence: bool);
}

pub type Receiver = watch::Receiver<bool>;
type Sender = watch::Sender<bool>;

#[derive(Debug)]
struct Presence {
    devices: HashMap<String, bool>,
    mqtt: MqttDeviceConfig,
    tx: Sender,
    overall_presence: Receiver,
}

impl Presence {
    fn build(mqtt: MqttDeviceConfig) -> Result<Self, MissingWildcard> {
        if !has_wildcards(&mqtt.topic) {
            return Err(MissingWildcard::new(&mqtt.topic));
        }

        let (tx, overall_presence) = watch::channel(false);
        Ok(Self {
            devices: HashMap::new(),
            overall_presence,
            mqtt,
            tx,
        })
    }
}

pub async fn start(
    mqtt: MqttDeviceConfig,
    mut mqtt_rx: mqtt::Receiver,
    client: AsyncClient,
) -> Result<Receiver, PresenceError> {
    // Subscribe to the relevant topics on mqtt
    client
        .subscribe(mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce)
        .await?;

    let mut presence = Presence::build(mqtt)?;
    let overall_presence = presence.overall_presence.clone();

    tokio::spawn(async move {
        loop {
            // TODO: Handle errors, warn if lagging
            if let Ok(message) = mqtt_rx.recv().await {
                presence.on_mqtt(&message).await;
            }
        }
    });

    Ok(overall_presence)
}

#[async_trait]
impl OnMqtt for Presence {
    async fn on_mqtt(&mut self, message: &rumqttc::Publish) {
        if !matches(&message.topic, &self.mqtt.topic) {
            return;
        }

        let offset = self
            .mqtt
            .topic
            .find('+')
            .or(self.mqtt.topic.find('#'))
            .expect("Presence::new fails if it does not contain wildcards");
        let device_name = &message.topic[offset..];

        if message.payload.is_empty() {
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
