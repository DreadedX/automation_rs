use serde::Deserialize;

use crate::traits;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    pub commands: Vec<Command>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub devices: Vec<Device>,
    pub execution: Vec<traits::Command>,
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

    use super::*;
    use crate::request::{Intent, Request};

    #[test]
    fn deserialize_set_fan_speed() {
        let req = json!({
          "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
          "inputs": [
            {
              "intent": "action.devices.EXECUTE",
              "payload": {
                "commands": [
                  {
                    "devices": [],
                    "execution": [
                      {
                        "command": "action.devices.commands.SetFanSpeed",
                        "params": {
                          "fanSpeed": "Test"
                        }
                      }
                    ]
                  }
                ]
              }
            }
          ]
        });

        let req: Request = serde_json::from_value(req).unwrap();

        assert_eq!(req.inputs.len(), 1);
        match &req.inputs[0] {
            Intent::Execute(payload) => {
                assert_eq!(payload.commands.len(), 1);
                assert_eq!(payload.commands[0].devices.len(), 0);
                assert_eq!(payload.commands[0].execution.len(), 1);
                match &payload.commands[0].execution[0] {
                    traits::Command::SetFanSpeed { fan_speed } => assert_eq!(fan_speed, "Test"),
                    _ => panic!("Expected SetFanSpeed"),
                }
            }
            _ => panic!("Expected Execute intent"),
        };
    }

    #[test]
    fn deserialize() {
        let req = json!({
          "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
          "inputs": [
            {
              "intent": "action.devices.EXECUTE",
              "payload": {
                "commands": [
                  {
                    "devices": [
                      {
                        "id": "123",
                        "customData": {
                          "fooValue": 74,
                          "barValue": true,
                          "bazValue": "sheepdip"
                        }
                      },
                      {
                        "id": "456",
                        "customData": {
                          "fooValue": 36,
                          "barValue": false,
                          "bazValue": "moarsheep"
                        }
                      }
                    ],
                    "execution": [
                      {
                        "command": "action.devices.commands.OnOff",
                        "params": {
                          "on": true
                        }
                      }
                    ]
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
            Intent::Execute(payload) => {
                assert_eq!(payload.commands.len(), 1);
                assert_eq!(payload.commands[0].devices.len(), 2);
                assert_eq!(payload.commands[0].devices[0].id, "123");
                assert_eq!(payload.commands[0].devices[1].id, "456");
                assert_eq!(payload.commands[0].execution.len(), 1);
                match payload.commands[0].execution[0] {
                    traits::Command::OnOff { on } => assert!(on),
                    _ => panic!("Expected OnOff"),
                }
            }
            _ => panic!("Expected Execute intent"),
        };
    }
}
