use serde::Serialize;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attributes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_only_on_off: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_only_on_off: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene_reversible: Option<bool>,
}
