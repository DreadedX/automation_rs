use async_trait::async_trait;
use automation_cast::Cast;
use serde::Serialize;

use crate::errors::{DeviceError, ErrorCode};
use crate::request::execute::CommandType;
use crate::response;
use crate::traits::{FanSpeed, HumiditySetting, OnOff, Scene, Trait};
use crate::types::Type;

#[async_trait]
pub trait GoogleHomeDevice:
    Sync + Send + Cast<dyn OnOff> + Cast<dyn Scene> + Cast<dyn FanSpeed> + Cast<dyn HumiditySetting>
{
    fn get_device_type(&self) -> Type;
    fn get_device_name(&self) -> Name;
    fn get_id(&self) -> String;
    fn is_online(&self) -> bool;

    // Default values that can optionally be overriden
    fn will_report_state(&self) -> bool {
        false
    }
    fn get_room_hint(&self) -> Option<&str> {
        None
    }
    fn get_device_info(&self) -> Option<Info> {
        None
    }

    async fn sync(&self) -> response::sync::Device {
        let name = self.get_device_name();
        let mut device =
            response::sync::Device::new(&self.get_id(), &name.name, self.get_device_type());

        device.name = name;
        device.will_report_state = self.will_report_state();
        // notification_supported_by_agent
        if let Some(room) = self.get_room_hint() {
            device.room_hint = Some(room.into());
        }
        device.device_info = self.get_device_info();

        let mut traits = Vec::new();

        // OnOff
        if let Some(on_off) = self.cast() as Option<&dyn OnOff> {
            traits.push(Trait::OnOff);
            device.attributes.command_only_on_off = on_off.is_command_only();
            device.attributes.query_only_on_off = on_off.is_query_only();
        }

        // Scene
        if let Some(scene) = self.cast() as Option<&dyn Scene> {
            traits.push(Trait::Scene);
            device.attributes.scene_reversible = scene.is_scene_reversible();
        }

        // FanSpeed
        if let Some(fan_speed) = self.cast() as Option<&dyn FanSpeed> {
            traits.push(Trait::FanSpeed);
            device.attributes.command_only_fan_speed = fan_speed.command_only_fan_speed();
            device.attributes.available_fan_speeds = Some(fan_speed.available_speeds());
        }

        if let Some(humidity_setting) = self.cast() as Option<&dyn HumiditySetting> {
            traits.push(Trait::HumiditySetting);
            device.attributes.query_only_humidity_setting =
                humidity_setting.query_only_humidity_setting();
        }

        device.traits = traits;

        device
    }

    async fn query(&self) -> response::query::Device {
        let mut device = response::query::Device::new();
        if !self.is_online() {
            device.set_offline();
        }

        // OnOff
        if let Some(on_off) = self.cast() as Option<&dyn OnOff> {
            device.state.on = on_off
                .is_on()
                .await
                .map_err(|err| device.set_error(err))
                .ok();
        }

        // FanSpeed
        if let Some(fan_speed) = self.cast() as Option<&dyn FanSpeed> {
            device.state.current_fan_speed_setting = Some(fan_speed.current_speed().await);
        }

        if let Some(humidity_setting) = self.cast() as Option<&dyn HumiditySetting> {
            device.state.humidity_ambient_percent =
                Some(humidity_setting.humidity_ambient_percent().await);
        }

        device
    }

    async fn execute(&mut self, command: &CommandType) -> Result<(), ErrorCode> {
        match command {
            CommandType::OnOff { on } => {
                if let Some(t) = self.cast_mut() as Option<&mut dyn OnOff> {
                    t.set_on(*on).await?;
                } else {
                    return Err(DeviceError::ActionNotAvailable.into());
                }
            }
            CommandType::ActivateScene { deactivate } => {
                if let Some(t) = self.cast_mut() as Option<&mut dyn Scene> {
                    t.set_active(!deactivate).await?;
                } else {
                    return Err(DeviceError::ActionNotAvailable.into());
                }
            }
            CommandType::SetFanSpeed { fan_speed } => {
                if let Some(t) = self.cast_mut() as Option<&mut dyn FanSpeed> {
                    t.set_speed(fan_speed).await?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Name {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    default_names: Vec<String>,
    name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    nicknames: Vec<String>,
}

impl Name {
    pub fn new(name: &str) -> Self {
        Self {
            default_names: Vec::new(),
            name: name.into(),
            nicknames: Vec::new(),
        }
    }

    pub fn add_default_name(&mut self, name: &str) {
        self.default_names.push(name.into());
    }

    pub fn add_nickname(&mut self, name: &str) {
        self.nicknames.push(name.into());
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hw_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sw_version: Option<String>,
    // attributes
    // customData
    // otherDeviceIds
}
