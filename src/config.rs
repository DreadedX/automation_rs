use std::{fs, error::Error, collections::HashMap};

use log::{debug, trace};
use rumqttc::AsyncClient;
use serde::Deserialize;

use crate::devices::{DeviceBox, IkeaOutlet, WakeOnLAN};

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
pub struct MqttDeviceConfig {
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
        mqtt: MqttDeviceConfig,
        kettle: Option<KettleConfig>,
    },
    WakeOnLAN {
        info: InfoConfig,
        mqtt: MqttDeviceConfig,
        mac_address: String,
    }
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

impl Device {
    pub fn into(self, identifier: String, client: AsyncClient) -> DeviceBox {
        match self {
            Device::IkeaOutlet { info, mqtt, kettle } => {
                trace!("\tIkeaOutlet [{} in {:?}]", info.name, info.room);
                Box::new(IkeaOutlet::new(identifier, info, mqtt, kettle, client))
            },
            Device::WakeOnLAN { info, mqtt, mac_address } => {
                trace!("\tWakeOnLan [{} in {:?}]", info.name, info.room);
                Box::new(WakeOnLAN::new(identifier, info, mqtt, mac_address, client))
            },
        }
    }
}
