use async_trait::async_trait;
use serde::Serialize;

use crate::errors::ErrorCode;

#[derive(Debug, Serialize)]
pub enum Trait {
    #[serde(rename = "action.devices.traits.OnOff")]
    OnOff,
    #[serde(rename = "action.devices.traits.Scene")]
    Scene,
    #[serde(rename = "action.devices.traits.FanSpeed")]
    FanSpeed,
    #[serde(rename = "action.devices.traits.HumiditySetting")]
    HumiditySetting,
}

#[async_trait]
#[impl_cast::device_trait]
pub trait OnOff {
    fn is_command_only(&self) -> Option<bool> {
        None
    }

    fn is_query_only(&self) -> Option<bool> {
        None
    }

    // TODO: Implement correct error so we can handle them properly
    async fn is_on(&self) -> Result<bool, ErrorCode>;
    async fn set_on(&mut self, on: bool) -> Result<(), ErrorCode>;
}

#[async_trait]
#[impl_cast::device_trait]
pub trait Scene {
    fn is_scene_reversible(&self) -> Option<bool> {
        None
    }

    async fn set_active(&self, activate: bool) -> Result<(), ErrorCode>;
}

#[derive(Debug, Serialize)]
pub struct SpeedValues {
    pub speed_synonym: Vec<String>,
    pub lang: String,
}

#[derive(Debug, Serialize)]
pub struct Speed {
    pub speed_name: String,
    pub speed_values: Vec<SpeedValues>,
}

#[derive(Debug, Serialize)]
pub struct AvailableSpeeds {
    pub speeds: Vec<Speed>,
    pub ordered: bool,
}

#[async_trait]
#[impl_cast::device_trait]
pub trait FanSpeed {
    fn reversible(&self) -> Option<bool> {
        None
    }

    fn command_only_fan_speed(&self) -> Option<bool> {
        None
    }

    fn available_speeds(&self) -> AvailableSpeeds;
    async fn current_speed(&self) -> String;
    async fn set_speed(&self, speed: &str) -> Result<(), ErrorCode>;
}

#[async_trait]
#[impl_cast::device_trait]
pub trait HumiditySetting {
    // TODO: This implementation is not complete, I have only implemented what I need right now
    fn query_only_humidity_setting(&self) -> Option<bool> {
        None
    }

    async fn humidity_ambient_percent(&self) -> isize;
}
