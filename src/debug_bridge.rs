use rumqttc::AsyncClient;
use serde::Deserialize;
use tracing::warn;

use crate::{
    event::{Event, EventChannel},
    mqtt::{DarknessMessage, PresenceMessage},
};

#[derive(Debug, Deserialize)]
pub struct DebugBridgeConfig {
    pub topic: String,
}

pub fn start(config: DebugBridgeConfig, event_channel: &EventChannel, client: AsyncClient) {
    let mut rx = event_channel.get_rx();

    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(Event::Presence(presence)) => {
                    let message = PresenceMessage::new(presence);
                    let topic = format!("{}/presence", config.topic);
                    client
                        .publish(
                            topic,
                            rumqttc::QoS::AtLeastOnce,
                            true,
                            serde_json::to_string(&message).unwrap(),
                        )
                        .await
                        .map_err(|err| {
                            warn!(
                                "Failed to update presence on {}/presence: {err}",
                                config.topic
                            )
                        })
                        .ok();
                }
                Ok(Event::Darkness(dark)) => {
                    let message = DarknessMessage::new(dark);
                    let topic = format!("{}/darkness", config.topic);
                    client
                        .publish(
                            topic,
                            rumqttc::QoS::AtLeastOnce,
                            true,
                            serde_json::to_string(&message).unwrap(),
                        )
                        .await
                        .map_err(|err| {
                            warn!(
                                "Failed to update presence on {}/presence: {err}",
                                config.topic
                            )
                        })
                        .ok();
                }
                Ok(_) => {}
                Err(_) => todo!("Handle errors with the event channel properly"),
            }
        }
    });
}
