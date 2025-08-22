#![allow(non_snake_case)]
use automation_cast::Cast;
use google_home_macro::traits;
use serde::Serialize;

use crate::errors::ErrorCode;
use crate::Device;

traits! {
    Device,
    "action.devices.traits.OnOff" => trait OnOff {
        command_only_on_off: Option<bool>,
        query_only_on_off: Option<bool>,
        async fn on(&self) -> Result<bool, ErrorCode>,
        "action.devices.commands.OnOff" => async fn set_on(&self, on: bool) -> Result<(), ErrorCode>,
    },
    "action.devices.traits.OpenClose" => trait OpenClose {
        discrete_only_open_close: Option<bool>,
        command_only_open_close: Option<bool>,
        query_only_open_close: Option<bool>,
        async fn open_percent(&self) -> Result<u8, ErrorCode>,
        "action.devices.commands.OpenClose" => async fn set_open_percent(&self, open_percent: u8) -> Result<(), ErrorCode>,
    },
    "action.devices.traits.Brightness" => trait Brightness {
        command_only_brightness: Option<bool>,
        async fn brightness(&self) -> Result<u8, ErrorCode>,
        "action.devices.commands.BrightnessAbsolute" => async fn set_brightness(&self, brightness: u8) -> Result<(), ErrorCode>,
    },
    "action.devices.traits.Scene" => trait Scene {
        scene_reversible: Option<bool>,

        "action.devices.commands.ActivateScene" => async fn set_active(&self, deactivate: bool) -> Result<(), ErrorCode>,
    },
    "action.devices.traits.FanSpeed" => trait FanSpeed {
        reversible: Option<bool>,
        command_only_fan_speed: Option<bool>,
        available_fan_speeds: AvailableSpeeds,

        async fn current_fan_speed_setting(&self) -> Result<String, ErrorCode>,

        // TODO: Figure out some syntax for optional command?
        // Probably better to just force the user to always implement commands?
        "action.devices.commands.SetFanSpeed" => async fn set_fan_speed(&self, fan_speed: String) -> Result<(), ErrorCode>,
    },
    "action.devices.traits.HumiditySetting" => trait HumiditySetting {
        query_only_humidity_setting: Option<bool>,

        async fn humidity_ambient_percent(&self) -> Result<isize, ErrorCode>,
    },
    "action.devices.traits.TemperatureControl" => trait TemperatureControl {
        query_only_temperature_control: Option<bool>,
        // TODO: Add rename
        temperatureUnitForUX: TemperatureUnit,

        async fn temperature_ambient_celsius(&self) -> Result<f32, ErrorCode>,
    }
}

#[derive(Debug, Serialize)]
pub struct SpeedValue {
    pub speed_synonym: Vec<String>,
    pub lang: String,
}

#[derive(Debug, Serialize)]
pub struct Speed {
    pub speed_name: String,
    pub speed_values: Vec<SpeedValue>,
}

#[derive(Debug, Serialize)]
pub struct AvailableSpeeds {
    pub speeds: Vec<Speed>,
    pub ordered: bool,
}

#[derive(Debug, Serialize)]
pub enum TemperatureUnit {
    #[serde(rename = "C")]
    Celsius,
    #[serde(rename = "F")]
    Fahrenheit,
}
