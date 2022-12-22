pub mod sync;
pub mod query;
pub mod execute;

use serde::Serialize;
use serde_with::skip_serializing_none;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    request_id: String,
    payload: ResponsePayload,
}

impl Response {
    pub fn new(request_id: String, payload: ResponsePayload) -> Self {
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

#[skip_serializing_none]
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub on: Option<bool>,
}
