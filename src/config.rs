use std::{fs, error::Error, collections::HashMap, net::{Ipv4Addr, SocketAddr}};

use regex::{Regex, Captures};
use tracing::{debug, trace, warn};
use rumqttc::AsyncClient;
use serde::Deserialize;

use crate::devices::{DeviceBox, IkeaOutlet, WakeOnLAN, AudioSetup, ContactSensor};

// @TODO Configure more defaults

#[derive(Debug, Deserialize)]
pub struct Config {
    pub openid: OpenIDConfig,
    pub mqtt: MqttConfig,
    #[serde(default)]
    pub fullfillment: FullfillmentConfig,
    pub ntfy: NtfyConfig,
    pub presence: MqttDeviceConfig,
    pub light_sensor: LightSensorConfig,
    pub hue_bridge: HueBridgeConfig,
    #[serde(default)]
    pub devices: HashMap<String, Device>
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenIDConfig {
    pub base_url: String
}

#[derive(Debug, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct FullfillmentConfig {
    #[serde(default = "default_fullfillment_ip")]
    pub ip: Ipv4Addr,
    #[serde(default = "default_fullfillment_port")]
    pub port: u16,
}

impl From<FullfillmentConfig> for SocketAddr {
    fn from(fullfillment: FullfillmentConfig) -> Self {
        (fullfillment.ip, fullfillment.port).into()
    }
}

impl Default for FullfillmentConfig {
    fn default() -> Self {
        Self { ip: default_fullfillment_ip(), port: default_fullfillment_port() }
    }
}

fn default_fullfillment_ip() -> Ipv4Addr {
    [127, 0, 0, 1].into()
}

fn default_fullfillment_port() -> u16 {
    7878
}

#[derive(Debug, Deserialize)]
pub struct NtfyConfig {
    #[serde(default = "default_ntfy_url")]
    pub url: String,
    pub topic: String,
}

fn default_ntfy_url() -> String {
    "https://ntfy.sh".into()
}

#[derive(Debug, Deserialize)]
pub struct LightSensorConfig {
    #[serde(flatten)]
    pub mqtt: MqttDeviceConfig,
    pub min: isize,
    pub max: isize,
}

#[derive(Debug, Deserialize)]
pub struct Flags {
    pub presence: isize,
    pub darkness: isize,
}

#[derive(Debug, Deserialize)]
pub struct HueBridgeConfig {
    pub ip: Ipv4Addr,
    pub login: String,
    pub flags: Flags,
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
pub struct PresenceDeviceConfig {
    #[serde(flatten)]
    pub mqtt: MqttDeviceConfig,
    // @TODO Maybe make this an option? That way if no timeout is set it will immediately turn the
    // device off again?
    pub timeout: u64 // Timeout in seconds
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Device {
    IkeaOutlet {
        #[serde(flatten)]
        info: InfoConfig,
        #[serde(flatten)]
        mqtt: MqttDeviceConfig,
        kettle: Option<KettleConfig>,
    },
    WakeOnLAN {
        #[serde(flatten)]
        info: InfoConfig,
        #[serde(flatten)]
        mqtt: MqttDeviceConfig,
        mac_address: String,
    },
    AudioSetup {
        #[serde(flatten)]
        mqtt: MqttDeviceConfig,
        mixer: Ipv4Addr,
        speakers: Ipv4Addr,
    },
    ContactSensor {
        #[serde(flatten)]
        mqtt: MqttDeviceConfig,
        presence: Option<PresenceDeviceConfig>,
    }
}

impl Config {
    pub fn build(filename: &str) -> Result<Self, Box<dyn Error>> {
        debug!("Loading config: {filename}");
        let file = fs::read_to_string(filename)?;

        // Substitute in environment variables
        let re = Regex::new(r"\$\{(.*)\}").unwrap();
        let file = re.replace_all(&file, |caps: &Captures| {
            let key = caps.get(1).unwrap().as_str();
            debug!("Substituting '{key}' in config");
            match std::env::var(key) {
                Ok(value) => value,
                Err(_) => {
                    // @TODO Would be nice if we could propagate this error upwards
                    warn!("Environment variable '{key}' is not set, using empty string as default");
                    "".to_string()
                }
            }
        });

        let config = toml::from_str(&file)?;
        Ok(config)
    }
}

impl Device {
    pub fn into(self, identifier: String, client: AsyncClient) -> DeviceBox {
        match self {
            Device::IkeaOutlet { info, mqtt, kettle } => {
                trace!(id = identifier, "IkeaOutlet [{} in {:?}]", info.name, info.room);
                Box::new(IkeaOutlet::new(identifier, info, mqtt, kettle, client))
            },
            Device::WakeOnLAN { info, mqtt, mac_address } => {
                trace!(id = identifier, "WakeOnLan [{} in {:?}]", info.name, info.room);
                Box::new(WakeOnLAN::new(identifier, info, mqtt, mac_address, client))
            },
            Device::AudioSetup { mqtt, mixer, speakers } => {
                trace!(id = identifier, "AudioSetup [{}]", identifier);
                Box::new(AudioSetup::new(identifier, mqtt, mixer, speakers, client))
            },
            Device::ContactSensor { mqtt, presence } => {
                trace!(id = identifier, "ContactSensor [{}]", identifier);
                Box::new(ContactSensor::new(identifier, mqtt, presence, client))
            },
        }
    }
}
