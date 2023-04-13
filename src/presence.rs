use std::collections::HashMap;

use async_trait::async_trait;
use rumqttc::{has_wildcards, matches, AsyncClient};
use serde::Deserialize;
use tracing::{debug, warn};

use crate::{
    config::MqttDeviceConfig,
    error::{MissingWildcard, PresenceError},
    event::{
        Event::{self, MqttMessage},
        EventChannel,
    },
    mqtt::PresenceMessage,
};

#[async_trait]
pub trait OnPresence: Sync + Send + 'static {
    async fn on_presence(&mut self, presence: bool);
}

#[derive(Debug, Deserialize)]
pub struct PresenceConfig {
    #[serde(flatten)]
    pub mqtt: MqttDeviceConfig,
}

const DEFAULT: bool = false;

pub async fn start(
    config: PresenceConfig,
    event_channel: &EventChannel,
    client: AsyncClient,
) -> Result<(), PresenceError> {
    if !has_wildcards(&config.mqtt.topic) {
        return Err(MissingWildcard::new(&config.mqtt.topic).into());
    }

    // Subscribe to the relevant topics on mqtt
    client
        .subscribe(config.mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce)
        .await?;

    let mut rx = event_channel.get_rx();
    let tx = event_channel.get_tx();

    let mut devices = HashMap::<String, bool>::new();
    let mut current_overall_presence = DEFAULT;

    tokio::spawn(async move {
        loop {
            // TODO: Handle errors, warn if lagging
            if let Ok(MqttMessage(message)) = rx.recv().await {
                if !matches(&message.topic, &config.mqtt.topic) {
                    continue;
                }

                let offset = config
                    .mqtt
                    .topic
                    .find('+')
                    .or(config.mqtt.topic.find('#'))
                    .expect("Presence::new fails if it does not contain wildcards");
                let device_name = message.topic[offset..].to_owned();

                if message.payload.is_empty() {
                    // Remove the device from the map
                    debug!("State of device [{device_name}] has been removed");
                    devices.remove(&device_name);
                } else {
                    let present = match PresenceMessage::try_from(message) {
                        Ok(state) => state.present(),
                        Err(err) => {
                            warn!("Failed to parse message: {err}");
                            continue;
                        }
                    };

                    debug!("State of device [{device_name}] has changed: {}", present);
                    devices.insert(device_name, present);
                }

                let overall_presence = devices.iter().any(|(_, v)| *v);
                if overall_presence != current_overall_presence {
                    debug!("Overall presence updated: {overall_presence}");
                    current_overall_presence = overall_presence;

                    if tx.send(Event::Presence(overall_presence)).is_err() {
                        warn!("There are no receivers on the event channel");
                    }
                }
            }
        }
    });

    Ok(())
}
