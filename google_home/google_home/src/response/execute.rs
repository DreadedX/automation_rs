use serde::Serialize;

use crate::errors::ErrorCode;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<ErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_string: Option<String>,
    commands: Vec<Command>,
}

impl Payload {
    pub fn new() -> Self {
        Self {
            error_code: None,
            debug_string: None,
            commands: Vec::new(),
        }
    }

    pub fn add_command(&mut self, command: Command) {
        if !command.is_empty() {
            self.commands.push(command);
        }
    }
}

impl Default for Payload {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<ErrorCode>,

    ids: Vec<String>,
    status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub states: Option<States>,
}

impl Command {
    pub fn new(status: Status) -> Self {
        Self {
            error_code: None,
            ids: Vec::new(),
            status,
            states: None,
        }
    }

    pub fn add_id(&mut self, id: &str) {
        self.ids.push(id.into());
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct States {
    pub online: bool,

    #[serde(flatten)]
    pub state: serde_json::Value,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Success,
    Pending,
    Offline,
    Exceptions,
    Error,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::errors::DeviceError;
    use crate::response::{Response, ResponsePayload};

    #[test]
    fn serialize() {
        let mut execute_resp = Payload::new();

        let state = json!({
            "on": true,
        });
        let mut command = Command::new(Status::Success);
        command.states = Some(States {
            online: true,
            state,
        });
        command.ids.push("123".into());
        execute_resp.add_command(command);

        let mut command = Command::new(Status::Error);
        command.error_code = Some(DeviceError::DeviceNotFound.into());
        command.ids.push("456".into());
        execute_resp.add_command(command);

        let resp = Response::new(
            "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
            ResponsePayload::Execute(execute_resp),
        );

        let resp = serde_json::to_value(resp).unwrap();

        let resp_expected = json!({
            "payload": {
                "commands": [
                    {
                        "states": {
                            "on": true,
                            "online": true
                        },
                        "ids": ["123"],
                        "status": "SUCCESS"
                    }, {
                        "errorCode": "deviceNotFound",
                        "ids": ["456"],
                        "status":"ERROR"
                    }
                ]
            },
            "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf"
        });

        assert_eq!(resp, resp_expected);
    }
}
