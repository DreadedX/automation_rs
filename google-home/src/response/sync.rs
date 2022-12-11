use serde::Serialize;

use crate::types::Type;
use crate::traits::Trait;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    user_agent_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    debug_string: Option<String>,
    devices: Vec<Device>,
}

impl Payload {
    pub fn new(user_agent_id: &str) -> Self {
        Self { user_agent_id: user_agent_id.into(), error_code: None, debug_string: None, devices: Vec::new() }
    }

    pub fn add_device(&mut self, device: Device) {
        self.devices.push(device);
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    #[serde(rename = "type")]
    pub device_type: Type,
    pub traits: Vec<Trait>,
    pub name: DeviceName,
    pub will_report_state: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_supported_by_agent: Option<bool>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub room_hint: String,
}

impl Device {
    pub fn new(id: &str, name: &str, device_type: Type) -> Self {
        Self {
            id: id.into(),
            device_type,
            traits: Vec::new(),
            name: DeviceName {
                default_name: Vec::new(),
                name: name.into(),
                nicknames: Vec::new() },
            will_report_state: true,
            notification_supported_by_agent: None,
            room_hint: "".into(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceName {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub default_name: Vec<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nicknames: Vec<String>,
}
