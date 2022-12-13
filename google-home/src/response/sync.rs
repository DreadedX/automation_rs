use serde::Serialize;

use crate::types::Type;
use crate::traits::Trait;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    agent_user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_string: Option<String>,
    pub devices: Vec<Device>,
}

impl Payload {
    pub fn new(agent_user_id: &str) -> Self {
        Self { agent_user_id: agent_user_id.into(), error_code: None, debug_string: None, devices: Vec::new() }
    }

    pub fn add_device(&mut self, device: Device) {
        self.devices.push(device);
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    id: String,
    #[serde(rename = "type")]
    device_type: Type,
    pub traits: Vec<Trait>,
    pub name: DeviceName,
    pub will_report_state: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_supported_by_agent: Option<bool>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub room_hint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_info: Option<DeviceInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Attributes>,
}

impl Device {
    pub fn new(id: &str, name: &str, device_type: Type) -> Self {
        Self {
            id: id.into(),
            device_type,
            traits: Vec::new(),
            name: DeviceName {
                default_names: Vec::new(),
                name: name.into(),
                nicknames: Vec::new() },
            will_report_state: false,
            notification_supported_by_agent: None,
            room_hint: "".into(),
            device_info: None,
            attributes: None,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceName {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub default_names: Vec<String>,
    name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nicknames: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub manufacturer: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub model: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub hw_version: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sw_version: String,
    // attributes
    // customData
    // otherDeviceIds
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attributes {

}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use uuid::Uuid;
    use super::*;
    use crate::{response::{Response, ResponsePayload}, types::Type, traits::Trait};

    #[test]
    fn serialize() {
        let mut sync_resp = Payload::new("1836.15267389");

        let mut device = Device::new("123", "Night light", Type::Kettle);
        device.traits.push(Trait::OnOff);
        device.name.default_names.push("My Outlet 1234".to_string());
        device.name.nicknames.push("wall plug".to_string());

        device.room_hint = "kitchen".into();
        device.device_info = Some(DeviceInfo {
            manufacturer: "lights-out-inc".to_string(),
            model: "hs1234".to_string(),
            hw_version: "3.2".to_string(),
            sw_version: "11.4".to_string(),
        });

        sync_resp.add_device(device);

        let resp = Response::new(Uuid::from_str("ff36a3cc-ec34-11e6-b1a0-64510650abcf").unwrap(), ResponsePayload::Sync(sync_resp));

        let json = serde_json::to_string(&resp).unwrap();

        println!("{}", json);

        assert_eq!(json, r#"{"requestId":"ff36a3cc-ec34-11e6-b1a0-64510650abcf","payload":{"agentUserId":"1836.15267389","devices":[{"id":"123","type":"action.devices.types.KETTLE","traits":["action.devices.traits.OnOff"],"name":{"defaultNames":["My Outlet 1234"],"name":"Night light","nicknames":["wall plug"]},"willReportState":false,"roomHint":"kitchen","deviceInfo":{"manufacturer":"lights-out-inc","model":"hs1234","hwVersion":"3.2","swVersion":"11.4"}}]}}"#)
    }
}
