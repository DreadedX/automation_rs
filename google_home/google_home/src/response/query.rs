use std::collections::HashMap;

use serde::Serialize;

use crate::errors::ErrorCode;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<ErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_string: Option<String>,
    pub devices: HashMap<String, Device>,
}

impl Payload {
    pub fn new() -> Self {
        Self {
            error_code: None,
            debug_string: None,
            devices: HashMap::new(),
        }
    }

    pub fn add_device(&mut self, id: &str, device: Device) {
        self.devices.insert(id.into(), device);
    }
}

impl Default for Payload {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Success,
    Offline,
    Exceptions,
    Error,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    online: bool,
    status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<ErrorCode>,

    #[serde(flatten)]
    pub state: serde_json::Value,
}

impl Device {
    pub fn new() -> Self {
        Self {
            online: true,
            status: Status::Success,
            error_code: None,
            state: Default::default(),
        }
    }

    pub fn set_offline(&mut self) {
        self.online = false;
        self.status = Status::Offline;
    }

    pub fn set_error(&mut self, err: ErrorCode) {
        self.status = match err {
            ErrorCode::DeviceError(_) => Status::Error,
            ErrorCode::DeviceException(_) => Status::Exceptions,
        };
        self.error_code = Some(err);
    }
}

impl Default for Device {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::response::{Response, ResponsePayload};

    #[test]
    fn serialize() {
        let mut query_resp = Payload::new();

        let mut device = Device::new();
        device.state = json!({
            "on": true,
        });
        query_resp.add_device("123", device);

        let mut device = Device::new();
        device.state = json!({
            "on": true,
        });
        query_resp.add_device("456", device);

        let resp = Response::new(
            "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
            ResponsePayload::Query(query_resp),
        );

        let json = serde_json::to_string(&resp).unwrap();

        println!("{}", json);

        // TODO: Add a known correct output to test against
    }
}
