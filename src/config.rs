use std::{fs, error::Error};

use log::debug;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub mqtt: MQTT,
}

#[derive(Debug, Deserialize)]
pub struct MQTT {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
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
