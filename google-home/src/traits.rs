use automation_cast::Cast;
use automation_macro::google_home_traits;
use serde::Serialize;

use crate::errors::ErrorCode;
use crate::GoogleHomeDevice;

google_home_traits! {
    GoogleHomeDevice,
    "action.devices.traits.OnOff" => trait OnOff {
        command_only_on_off: Option<bool>,
        query_only_on_off: Option<bool>,
        async fn on(&self) -> Result<bool, ErrorCode>,
        "action.devices.commands.OnOff" => async fn set_on(&mut self, on: bool) -> Result<(), ErrorCode>,
    },
    "action.devices.traits.Scene" => trait Scene {
        scene_reversible: Option<bool>,

        "action.devices.commands.ActivateScene" => async fn set_active(&mut self, activate: bool) -> Result<(), ErrorCode>,
    },
    "action.devices.traits.FanSpeed" => trait FanSpeed {
        reversible: Option<bool>,
        command_only_fan_speed: Option<bool>,
        available_fan_speeds: AvailableSpeeds,

        fn current_fan_speed_setting(&self) -> Result<String, ErrorCode>,

        // TODO: Figure out some syntax for optional command?
        // Probably better to just force the user to always implement commands?
        "action.devices.commands.SetFanSpeed" => async fn set_fan_speed(&mut self, fan_speed: String) -> Result<(), ErrorCode>,
    },
    "action.devices.traits.HumiditySetting" => trait HumiditySetting {
        query_only_humidity_setting: Option<bool>,

        fn humidity_ambient_percent(&self) -> Result<isize, ErrorCode>,
    }
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
