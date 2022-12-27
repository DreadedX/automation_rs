use std::{fs, error::Error, collections::HashMap};

use log::debug;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub mqtt: MQTTConfig,
    pub fullfillment: FullfillmentConfig,
    #[serde(default)]
    pub devices: HashMap<String, Device>
}

#[derive(Debug, Deserialize)]
pub struct MQTTConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FullfillmentConfig {
    pub port: u16,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct InfoConfig {
    pub name: String,
    pub room: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ZigbeeDeviceConfig {
    pub topic: String,
}

#[derive(Debug, Deserialize)]
pub struct KettleConfig {
    pub timeout: Option<u64>, // Timeout in seconds
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Device {
    IkeaOutlet {
        info: InfoConfig,
        zigbee: ZigbeeDeviceConfig,
        kettle: Option<KettleConfig>,
    },
}

impl Config {
    pub fn build(filename: &str) -> Result<Self, Box<dyn Error>> {
        debug!("Loading config: {filename}");
        let file = fs::read_to_string(filename)?;
        let mut config: Self = toml::from_str(&file)?;

        config.mqtt.password = Some(std::env::var("MQTT_PASSWORD").or(config.mqtt.password.ok_or("MQTT password needs to be set in either config or the environment!"))?);

        Ok(config)
    }
}
