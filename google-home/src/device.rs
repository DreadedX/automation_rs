use serde::Serialize;

use crate::{response, types::Type, traits::{AsOnOff, Trait, AsScene}, errors::{DeviceError, ErrorCode}, request::execute::CommandType};

pub trait GoogleHomeDevice: AsOnOff + AsScene {
    fn get_device_type(&self) -> Type;
    fn get_device_name(&self) -> Name;
    fn get_id(&self) -> &str;
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



    fn sync(&self) -> response::sync::Device {
        let name = self.get_device_name();
        let mut device = response::sync::Device::new(&self.get_id(), &name.name, self.get_device_type());

        device.name = name;
        device.will_report_state = self.will_report_state();
        // notification_supported_by_agent
        if let Some(room) = self.get_room_hint() {
            device.room_hint = Some(room.into());
        }
        device.device_info = self.get_device_info();

        let mut traits = Vec::new();
        // OnOff
        if let Some(d) = AsOnOff::cast(self) {
            traits.push(Trait::OnOff);
            device.attributes.command_only_on_off = d.is_command_only();
            device.attributes.query_only_on_off = d.is_query_only();
        }

        // Scene
        if let Some(d) = AsScene::cast(self) {
            traits.push(Trait::Scene);
            device.attributes.scene_reversible = d.is_scene_reversible();
        }

        device.traits = traits;

        return device;
    }

    fn query(&self) -> response::query::Device {
        let mut device = response::query::Device::new();
        if !self.is_online() {
            device.set_offline();
        }

        // OnOff
        if let Some(d) = AsOnOff::cast(self) {
            match d.is_on() {
                Ok(state) => device.state.on = Some(state),
                Err(err) => device.set_error(err),
            }
        }

        return device;
    }

    fn execute(&mut self, command: &CommandType) -> Result<(), ErrorCode> {
        match command {
            CommandType::OnOff { on } => {
                if let Some(d) = AsOnOff::cast_mut(self) {
                    d.set_on(*on)?;
                } else {
                    return Err(DeviceError::ActionNotAvailable.into());
                }
            },
            CommandType::ActivateScene { deactivate } => {
                if let Some(d) = AsScene::cast_mut(self) {
                    d.set_active(!deactivate)?;
                } else {
                    return Err(DeviceError::ActionNotAvailable.into());
                }
            },
        }

        return Ok(());
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
        Self { default_names: Vec::new(), name: name.into(), nicknames: Vec::new() }
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
