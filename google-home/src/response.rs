pub mod sync;

use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    request_id: Uuid,
    payload: ResponsePayload,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ResponsePayload {
    Sync(sync::Payload)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::{response::sync::Device, types::Type, traits::Trait};

    use super::*;

    #[test]
    fn serialize_sync_response() {
        let mut sync_resp = sync::Payload::new("Dreaded_X");

        let mut device = Device::new("kitchen/kettle", "Kettle", Type::Kettle);
        device.traits.push(Trait::OnOff);
        device.room_hint = "Kitchen".into();
        sync_resp.add_device(device);

        let resp = Response{ request_id: Uuid::from_str("ff36a3cc-ec34-11e6-b1a0-64510650abcf").unwrap(), payload: ResponsePayload::Sync(sync_resp) };

        println!("{:?}", resp);

        let json = serde_json::to_string(&resp).unwrap();

        println!("{}", json);
    }
}
