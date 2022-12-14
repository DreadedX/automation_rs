pub mod sync;
pub mod query;
pub mod execute;

use serde::Deserialize;
use uuid::Uuid;

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
    pub request_id: Uuid,
    pub inputs: Vec<Intent>,
}
