use serde::Serialize;
use serde_with::skip_serializing_none;

use crate::attributes::Attributes;
use crate::device;
use crate::errors::ErrorCode;
use crate::types::Type;
use crate::traits::Trait;

#[skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    agent_user_id: String,
    pub error_code: Option<ErrorCode>,
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

#[skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    id: String,
    #[serde(rename = "type")]
    device_type: Type,
    pub traits: Vec<Trait>,
    pub name: device::Name,
    pub will_report_state: bool,
    pub notification_supported_by_agent: Option<bool>,
    pub room_hint: Option<String>,
    pub device_info: Option<device::Info>,
    pub attributes: Attributes,
}

impl Device {
    pub fn new(id: &str, name: &str, device_type: Type) -> Self {
        Self {
            id: id.into(),
            device_type,
            traits: Vec::new(),
            name: device::Name::new(name),
            will_report_state: false,
            notification_supported_by_agent: None,
            room_hint: None,
            device_info: None,
            attributes: Attributes::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{response::{Response, ResponsePayload}, types::Type, traits::Trait};

    #[test]
    fn serialize() {
        let mut sync_resp = Payload::new("1836.15267389");

        let mut device = Device::new("123", "Night light", Type::Kettle);
        device.traits.push(Trait::OnOff);
        device.name.add_default_name("My Outlet 1234");
        device.name.add_nickname("wall plug");

        device.room_hint = Some("kitchen".into());
        device.device_info = Some(device::Info {
            manufacturer: Some("lights-out-inc".to_string()),
            model: Some("hs1234".to_string()),
            hw_version: Some("3.2".to_string()),
            sw_version: Some("11.4".to_string()),
        });

        sync_resp.add_device(device);

        let resp = Response::new("ff36a3cc-ec34-11e6-b1a0-64510650abcf".to_owned(), ResponsePayload::Sync(sync_resp));

        let json = serde_json::to_string(&resp).unwrap();

        println!("{}", json);

        // assert_eq!(json, r#"{"requestId":"ff36a3cc-ec34-11e6-b1a0-64510650abcf","payload":{"agentUserId":"1836.15267389","devices":[{"id":"123","type":"action.devices.types.KETTLE","traits":["action.devices.traits.OnOff"],"name":{"defaultNames":["My Outlet 1234"],"name":"Night light","nicknames":["wall plug"]},"willReportState":false,"roomHint":"kitchen","deviceInfo":{"manufacturer":"lights-out-inc","model":"hs1234","hwVersion":"3.2","swVersion":"11.4"}}]}}"#)
    }
}
