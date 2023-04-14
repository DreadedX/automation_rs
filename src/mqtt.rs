use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, warn};

use rumqttc::{Event, EventLoop, Incoming, Publish};

use crate::event::{self, EventChannel};

#[async_trait]
#[impl_cast::device_trait]
pub trait OnMqtt {
    fn topics(&self) -> Vec<&str>;
    async fn on_mqtt(&mut self, message: Publish);
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid message payload received: {0:?}")]
    InvalidPayload(Bytes),
}

pub fn start(mut eventloop: EventLoop, event_channel: &EventChannel) {
    let tx = event_channel.get_tx();

    tokio::spawn(async move {
        debug!("Listening for MQTT events");
        loop {
            let notification = eventloop.poll().await;
            match notification {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    tx.send(event::Event::MqttMessage(p)).await.ok();
                }
                Ok(..) => continue,
                Err(err) => {
                    // Something has gone wrong
                    // We stay in the loop as that will attempt to reconnect
                    warn!("{}", err);
                }
            }
        }
    });
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OnOffMessage {
    state: String,
}

impl OnOffMessage {
    pub fn new(state: bool) -> Self {
        Self {
            state: if state { "ON" } else { "OFF" }.into(),
        }
    }

    pub fn state(&self) -> bool {
        self.state == "ON"
    }
}

impl TryFrom<Publish> for OnOffMessage {
    type Error = ParseError;

    fn try_from(message: Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(ParseError::InvalidPayload(message.payload.clone())))
    }
}

#[derive(Debug, Deserialize)]
pub struct ActivateMessage {
    activate: bool,
}

impl ActivateMessage {
    pub fn activate(&self) -> bool {
        self.activate
    }
}

impl TryFrom<Publish> for ActivateMessage {
    type Error = ParseError;

    fn try_from(message: Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(ParseError::InvalidPayload(message.payload.clone())))
    }
}

#[derive(Debug, Deserialize, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RemoteAction {
    On,
    Off,
    BrightnessMoveUp,
    BrightnessMoveDown,
    BrightnessStop,
}

#[derive(Debug, Deserialize)]
pub struct RemoteMessage {
    action: RemoteAction,
}

impl RemoteMessage {
    pub fn action(&self) -> RemoteAction {
        self.action
    }
}

impl TryFrom<Publish> for RemoteMessage {
    type Error = ParseError;

    fn try_from(message: Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(ParseError::InvalidPayload(message.payload.clone())))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PresenceMessage {
    state: bool,
    updated: Option<u128>,
}

impl PresenceMessage {
    pub fn new(state: bool) -> Self {
        Self {
            state,
            updated: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time is after UNIX EPOCH")
                    .as_millis(),
            ),
        }
    }

    pub fn present(&self) -> bool {
        self.state
    }
}

impl TryFrom<Publish> for PresenceMessage {
    type Error = ParseError;

    fn try_from(message: Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(ParseError::InvalidPayload(message.payload.clone())))
    }
}

#[derive(Debug, Deserialize)]
pub struct BrightnessMessage {
    illuminance: isize,
}

impl BrightnessMessage {
    pub fn illuminance(&self) -> isize {
        self.illuminance
    }
}

impl TryFrom<Publish> for BrightnessMessage {
    type Error = ParseError;

    fn try_from(message: Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(ParseError::InvalidPayload(message.payload.clone())))
    }
}

#[derive(Debug, Deserialize)]
pub struct ContactMessage {
    contact: bool,
}

impl ContactMessage {
    pub fn is_closed(&self) -> bool {
        self.contact
    }
}

impl TryFrom<Publish> for ContactMessage {
    type Error = ParseError;

    fn try_from(message: Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(ParseError::InvalidPayload(message.payload.clone())))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DarknessMessage {
    state: bool,
    updated: Option<u128>,
}

impl DarknessMessage {
    pub fn new(state: bool) -> Self {
        Self {
            state,
            updated: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time is after UNIX EPOCH")
                    .as_millis(),
            ),
        }
    }

    pub fn present(&self) -> bool {
        self.state
    }
}

impl TryFrom<Publish> for DarknessMessage {
    type Error = ParseError;

    fn try_from(message: Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(ParseError::InvalidPayload(message.payload.clone())))
    }
}
