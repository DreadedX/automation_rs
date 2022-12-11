use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "intent")]
enum Intent {
    #[serde(rename = "action.devices.SYNC")]
    Sync,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    request_id: Uuid,
    inputs: Vec<Intent>,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn deserialize_sync_request() {

        let json = r#"{
      "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
      "inputs": [
        {
          "intent": "action.devices.SYNC"
        }
      ]
    }"#;

        let req: Request = serde_json::from_str(json).unwrap();

        assert_eq!(req.request_id, Uuid::from_str("ff36a3cc-ec34-11e6-b1a0-64510650abcf").unwrap());
        assert_eq!(req.inputs.len(), 1);
        assert_eq!(req.inputs[0], Intent::Sync);
    }
}

