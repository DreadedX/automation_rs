use serde::Serialize;
use serde_with::skip_serializing_none;

use crate::{response, types::Type, traits::{AsOnOff, Trait, AsScene}};

pub trait GoogleHomeDevice: AsOnOff + AsScene {
    fn get_device_type(&self) -> Type;
    fn get_device_name(&self) -> Name;
    fn get_id(&self) -> &str;

    // Default values that can optionally be overriden
    fn will_report_state(&self) -> bool {
        false
    }
    fn get_room_hint(&self) -> Option<String> {
        None
    }
    fn get_device_info(&self) -> Option<Info> {
        None
    }
}

// This trait exists just to hide the sync, query and execute function from the user
pub trait GoogleHomeDeviceFullfillment: GoogleHomeDevice {
    fn sync(&self) -> response::sync::Device {
        let name = self.get_device_name();
        let mut device = response::sync::Device::new(&self.get_id(), &name.name, self.get_device_type());

        device.name = name;
        device.will_report_state = self.will_report_state();
        // notification_supported_by_agent
        device.room_hint = self.get_room_hint();
        device.device_info = self.get_device_info();

        let mut traits = Vec::new();
        // OnOff
        {
            if let Some(d) = AsOnOff::cast(self) {
                traits.push(Trait::OnOff);
                device.attributes.command_only_on_off = d.is_command_only();
                device.attributes.query_only_on_off = d.is_query_only();
            }
        }

        // Scene
        {
            if let Some(d) = AsScene::cast(self) {
                traits.push(Trait::Scene);
                device.attributes.scene_reversible = d.is_scene_reversible();
            }
        }

        device.traits = traits;

        return device;
    }
}

impl<T: GoogleHomeDevice> GoogleHomeDeviceFullfillment for T {}

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

#[skip_serializing_none]
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub hw_version: Option<String>,
    pub sw_version: Option<String>,
    // attributes
    // customData
    // otherDeviceIds
}
