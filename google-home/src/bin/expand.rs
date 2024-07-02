use std::error::Error;

use automation_cast::Cast;
use automation_macro::google_home_traits;
use google_home::errors::ErrorCode;
use google_home::traits::AvailableSpeeds;

trait GoogleHomeDevice: GoogleHomeDeviceFulfillment {}
google_home_traits! {
    GoogleHomeDevice,
    "action.devices.traits.OnOff" => trait OnOff {
        command_only_on_off: Option<bool>,
        query_only_on_off: Option<bool>,
        async fn on(&self) -> Result<bool, ErrorCode>,
        "action.devices.commands.OnOff" => async fn set_on(&self, on: bool) -> Result<(), ErrorCode>,
    },
    "action.devices.traits.Scene" => trait Scene {
        scene_reversible: Option<bool>,

        "action.devices.commands.ActivateScene" => async fn set_active(&self, activate: bool) -> Result<(), ErrorCode>,
    },
    "action.devices.traits.FanSpeed" => trait FanSpeed {
        reversible: Option<bool>,
        command_only_fan_speed: Option<bool>,
        available_fan_speeds: AvailableSpeeds,

        fn current_fan_speed_setting(&self) -> Result<String, ErrorCode>,
        fn current_fan_speed_percent(&self) -> Result<String, ErrorCode>,

        // TODO: Figure out some syntax for optional command?
        // Probably better to just force the user to always implement commands?
        "action.devices.commands.SetFanSpeed" => fn set_fan_speed(&self, fan_speed: String),
    },
    "action.devices.traits.HumiditySetting" => trait HumiditySetting {
        query_only_humidity_setting: Option<bool>,

        fn humidity_ambient_percent(&self) -> Result<Option<isize>, ErrorCode>,
    }
}

struct Device {}
impl GoogleHomeDevice for Device {}

#[async_trait::async_trait]
impl OnOff for Device {
    fn command_only_on_off(&self) -> Option<bool> {
        Some(true)
    }

    async fn on(&self) -> Result<bool, ErrorCode> {
        Ok(true)
    }

    async fn set_on(&self, _on: bool) -> Result<(), ErrorCode> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl HumiditySetting for Device {
    fn query_only_humidity_setting(&self) -> Option<bool> {
        Some(true)
    }

    fn humidity_ambient_percent(&self) -> Result<Option<isize>, ErrorCode> {
        Ok(Some(44))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let device: Box<dyn GoogleHomeDevice> = Box::new(Device {});

    let (traits, sync) = device.sync().await?;
    let query = device.query().await?;

    println!("{traits:?}");
    println!("{sync}");
    println!("{query}");

    let state = device.execute(Command::OnOff { on: true }).await?;

    println!("{state}");

    Ok(())
}
