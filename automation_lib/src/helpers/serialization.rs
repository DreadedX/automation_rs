use serde::de::{self, Unexpected};
use serde::{Deserialize, Deserializer};

pub fn state_deserializer<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match String::deserialize(deserializer)?.as_ref() {
        "ON" => Ok(true),
        "OFF" => Ok(false),
        other => Err(de::Error::invalid_value(
            Unexpected::Str(other),
            &"Value expected was either ON or OFF",
        )),
    }
}
