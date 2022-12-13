pub mod sync;
pub mod query;
pub mod execute;

use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    request_id: Uuid,
    payload: ResponsePayload,
}

impl Response {
    fn new(request_id: Uuid, payload: ResponsePayload) -> Self {
        Self { request_id, payload }
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ResponsePayload {
    Sync(sync::Payload),
    Query(query::Payload),
    Execute(execute::Payload),
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    #[serde(skip_serializing_if = "Option::is_none")]
    on: Option<bool>,
}

impl State {
    fn on(mut self, state: bool) -> Self {
        self.on = Some(state);
        self
    }
}

