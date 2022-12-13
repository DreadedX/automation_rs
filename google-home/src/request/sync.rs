#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use uuid::Uuid;

    use crate::request::{Request, Intent};

    #[test]
    fn deserialize() {

        let json = r#"{
      "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
      "inputs": [
        {
          "intent": "action.devices.SYNC"
        }
      ]
    }"#;

        let req: Request = serde_json::from_str(json).unwrap();

        println!("{:?}", req);

        assert_eq!(req.request_id, Uuid::from_str("ff36a3cc-ec34-11e6-b1a0-64510650abcf").unwrap());
        assert_eq!(req.inputs.len(), 1);
        match req.inputs[0] {
            Intent::Sync => {},
            _ => panic!("Expected Sync intent")
        }
    }
}

