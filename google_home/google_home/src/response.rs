pub mod execute;
pub mod query;
pub mod sync;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    request_id: String,
    payload: ResponsePayload,
}

impl Response {
    pub fn new(request_id: &str, payload: ResponsePayload) -> Self {
        Self {
            request_id: request_id.into(),
            payload,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ResponsePayload {
    Sync(sync::Payload),
    Query(query::Payload),
    Execute(execute::Payload),
}
