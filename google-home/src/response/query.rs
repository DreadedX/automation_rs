use std::collections::HashMap;

use serde::Serialize;
use serde_with::skip_serializing_none;

use crate::{response::State, errors::Errors};

#[skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    pub error_code: Option<Errors>,
    pub debug_string: Option<String>,
    devices: HashMap<String, Device>,
}

impl Payload {
    pub fn new() -> Self {
        Self { error_code: None, debug_string: None, devices: HashMap::new() }
    }

    pub fn add_device(&mut self, id: &str, device: Device) {
        self.devices.insert(id.into(), device);
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

#[skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    online: bool,
    status: Status,
    pub error_code: Option<Errors>,

    #[serde(flatten)]
    pub state: State,
}

impl Device {
    pub fn new(online: bool, status: Status) -> Self {
        Self { online, status, error_code: None, state: State::default() }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use uuid::Uuid;
    use super::*;
    use crate::response::{Response, ResponsePayload, State};

    #[test]
    fn serialize() {
        let mut query_resp = Payload::new();

        let state = State::default();
        let mut device = Device::new(true, Status::Success);
        device.state.on = Some(true);
        query_resp.add_device("123", device);

        let state = State::default();
        let mut device = Device::new(true, Status::Success);
        device.state.on = Some(false);
        query_resp.add_device("456", device);

        let resp = Response::new(Uuid::from_str("ff36a3cc-ec34-11e6-b1a0-64510650abcf").unwrap(), ResponsePayload::Query(query_resp));

        let json = serde_json::to_string(&resp).unwrap();

        println!("{}", json);

        // @TODO Add a known correct output to test against
    }
}
