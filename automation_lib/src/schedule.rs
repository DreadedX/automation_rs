use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize, Hash, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    On,
    Off,
}

pub type Schedule = IndexMap<String, IndexMap<Action, Vec<String>>>;

// #[derive(Debug, Deserialize)]
// pub struct Schedule {
//     pub when: String,
//     pub actions: IndexMap<Action, Vec<String>>,
// }
