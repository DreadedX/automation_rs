pub mod execute;
pub mod query;
pub mod sync;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "intent", content = "payload")]
pub enum Intent {
    #[serde(rename = "action.devices.SYNC")]
    Sync,
    #[serde(rename = "action.devices.QUERY")]
    Query(query::Payload),
    #[serde(rename = "action.devices.EXECUTE")]
    Execute(execute::Payload),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub request_id: String,
    pub inputs: Vec<Intent>,
}
