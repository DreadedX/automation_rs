use serde::Serialize;
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attributes {
    pub command_only_on_off: Option<bool>,
    pub query_only_on_off: Option<bool>,
    pub scene_reversible: Option<bool>,
}
