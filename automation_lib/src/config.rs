use lua_typed::Typed;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Typed)]
pub struct InfoConfig {
    pub name: String,
    pub room: Option<String>,
}

impl InfoConfig {
    pub fn identifier(&self) -> String {
        (if let Some(room) = &self.room {
            room.to_ascii_lowercase().replace(' ', "_") + "_"
        } else {
            String::new()
        }) + &self.name.to_ascii_lowercase().replace(' ', "_")
    }
}

#[derive(Debug, Clone, Deserialize, Typed)]
pub struct MqttDeviceConfig {
    pub topic: String,
}
