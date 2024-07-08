use serde::Serialize;

use crate::device;
use crate::errors::ErrorCode;
use crate::traits::Trait;
use crate::types::Type;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    agent_user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<ErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_string: Option<String>,
    pub devices: Vec<Device>,
}

impl Payload {
    pub fn new(agent_user_id: &str) -> Self {
        Self {
            agent_user_id: agent_user_id.into(),
            error_code: None,
            debug_string: None,
            devices: Vec::new(),
        }
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
    pub name: device::Name,
    pub will_report_state: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_supported_by_agent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_info: Option<device::Info>,
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub attributes: serde_json::Value,
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
            attributes: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::response::{Response, ResponsePayload};
    use crate::traits::Trait;
    use crate::types::Type;

    #[test]
    fn serialize() {
        let mut sync_resp = Payload::new("1836.15267389");

        let mut device = Device::new("123", "Night light", Type::Kettle);
        device.traits.push(Trait::OnOff);
        device.name.add_default_name("My Outlet 1234");
        device.name.add_nickname("wall plug");

        device.room_hint = Some("kitchen".into());
        device.device_info = Some(device::Info {
            manufacturer: Some("lights-out-inc".into()),
            model: Some("hs1234".into()),
            hw_version: Some("3.2".into()),
            sw_version: Some("11.4".into()),
        });

        sync_resp.add_device(device);

        let resp = Response::new(
            "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
            ResponsePayload::Sync(sync_resp),
        );

        let resp = serde_json::to_value(resp).unwrap();

        let resp_expected = json!({
            "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
            "payload": {
                "agentUserId": "1836.15267389",
                "devices": [
                    {
                        "id": "123",
                        "type": "action.devices.types.KETTLE",
                        "traits": ["action.devices.traits.OnOff"],
                        "name": {
                            "defaultNames": ["My Outlet 1234"],
                            "name": "Night light",
                            "nicknames": ["wall plug"]
                        },
                        "willReportState": false,
                        "roomHint": "kitchen",
                        "deviceInfo": {
                            "manufacturer": "lights-out-inc",
                            "model": "hs1234",
                            "hwVersion": "3.2",
                            "swVersion": "11.4"
                        }
                    }
                ]
            }
        });

        assert_eq!(resp, resp_expected);
    }
}
