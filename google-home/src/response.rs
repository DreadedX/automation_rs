pub mod sync;
pub mod query;
pub mod execute;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    request_id: String,
    payload: ResponsePayload,
}

impl Response {
    pub fn new(request_id: &str, payload: ResponsePayload) -> Self {
        Self { request_id: request_id.to_owned(), payload }
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
    pub on: Option<bool>,
}
