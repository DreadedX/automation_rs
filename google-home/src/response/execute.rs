use serde::Serialize;
use serde_with::skip_serializing_none;

use crate::response::State;

#[skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    pub error_code: Option<String>,
    pub debug_string: Option<String>,
    commands: Vec<Command>,
}

impl Payload {
    pub fn new() -> Self {
        Self { error_code: None, debug_string: None, commands: Vec::new() }
    }

    pub fn add_command(&mut self, command: Command) {
        self.commands.push(command);
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub error_code: Option<String>,

    ids: Vec<String>,
    status: Status,
    pub states: Option<States>,
}

impl Command {
    pub fn new(status: Status) -> Self {
        Self { error_code: None, ids: Vec::new(), status, states: None }
    }

    pub fn add_id(&mut self, id: &str) {
        self.ids.push(id.into());
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct States {
    pub online: bool,

    #[serde(flatten)]
    pub state: Option<State>,
}

#[derive(Debug, Serialize)]
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
    use std::str::FromStr;
    use uuid::Uuid;
    use super::*;
    use crate::response::{Response, ResponsePayload, State};

    #[test]
    fn serialize() {
        let mut execute_resp = Payload::new();

        let state = State::default().on(true);
        let mut command = Command::new(Status::Success);
        command.states = Some(States {
            online: true,
            state: Some(state)
        });
        command.ids.push("123".into());
        execute_resp.add_command(command);

        let mut command = Command::new(Status::Error);
        command.error_code = Some("deviceTurnedOff".into());
        command.ids.push("456".into());
        execute_resp.add_command(command);

        let resp = Response::new(Uuid::from_str("ff36a3cc-ec34-11e6-b1a0-64510650abcf").unwrap(), ResponsePayload::Execute(execute_resp));

        let json = serde_json::to_string(&resp).unwrap();

        println!("{}", json);

        // @TODO Add a known correct output to test against
    }
}
