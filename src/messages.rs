use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use rumqttc::Publish;
use serde::{Deserialize, Serialize};

use crate::error::ParseError;

// Message used to turn on and off devices and receiving their state
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

// Message send to request activating a device
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

// Actions that can be performed by a remote
#[derive(Debug, Deserialize, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RemoteAction {
    On,
    Off,
    BrightnessMoveUp,
    BrightnessMoveDown,
    BrightnessStop,
}

// Message used to report the action performed by a remote
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

// Message used to report the current presence state
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

    pub fn presence(&self) -> bool {
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

// Message used to report the state of a light sensor
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

// Message to report the state of a contact sensor
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

// Message used to report the current darkness state
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

    pub fn is_dark(&self) -> bool {
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

// Message used to report the power draw a smart plug
#[derive(Debug, Deserialize)]
pub struct PowerMessage {
    power: f32,
}

impl PowerMessage {
    pub fn power(&self) -> f32 {
        self.power
    }
}

impl TryFrom<Publish> for PowerMessage {
    type Error = ParseError;

    fn try_from(message: Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(ParseError::InvalidPayload(message.payload.clone())))
    }
}

// Message used to report the power state of a hue light
#[derive(Debug, Deserialize)]
pub struct HueState {
    on: bool,
}

#[derive(Debug, Deserialize)]
pub struct HueMessage {
    state: HueState,
}

impl HueMessage {
    pub fn is_on(&self) -> bool {
        self.state.on
    }
}

impl TryFrom<Bytes> for HueMessage {
    type Error = ParseError;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        serde_json::from_slice(&bytes).or(Err(ParseError::InvalidPayload(bytes.clone())))
    }
}

// TODO: Import this from the air_filter code itself instead of copying
#[derive(PartialEq, Eq, Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AirFilterState {
    Off,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct AirFilterMessage {
    state: AirFilterState,
}

impl AirFilterMessage {
    pub fn state(&self) -> AirFilterState {
        self.state
    }

    pub fn new(state: AirFilterState) -> Self {
        Self { state }
    }
}

impl TryFrom<Publish> for AirFilterMessage {
    type Error = ParseError;

    fn try_from(message: Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(ParseError::InvalidPayload(message.payload.clone())))
    }
}
