use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    pub devices: Vec<Device>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    // customData
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::request::{Intent, Request};

    #[test]
    fn deserialize() {
        let req = json!({
          "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
          "inputs": [
            {
              "intent": "action.devices.QUERY",
              "payload": {
                "devices": [
                  {
                    "id": "123",
                    "customData": {
                      "fooValue": 74,
                      "barValue": true,
                      "bazValue": "foo"
                    }
                  },
                  {
                    "id": "456",
                    "customData": {
                      "fooValue": 12,
                      "barValue": false,
                      "bazValue": "bar"
                    }
                  }
                ]
              }
            }
          ]
        });

        let req: Request = serde_json::from_value(req).unwrap();

        println!("{:?}", req);

        assert_eq!(
            req.request_id,
            "ff36a3cc-ec34-11e6-b1a0-64510650abcf".to_string()
        );
        assert_eq!(req.inputs.len(), 1);
        match &req.inputs[0] {
            Intent::Query(payload) => {
                assert_eq!(payload.devices.len(), 2);
                assert_eq!(payload.devices[0].id, "123");
                assert_eq!(payload.devices[1].id, "456");
            }
            _ => panic!("Expected Query intent"),
        };
    }
}
