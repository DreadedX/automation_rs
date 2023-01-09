use std::{fs, error::Error, net::{Ipv4Addr, SocketAddr}, collections::HashMap};

use async_recursion::async_recursion;
use regex::{Regex, Captures};
use tracing::{debug, trace, error};
use rumqttc::{AsyncClient, has_wildcards};
use serde::Deserialize;

use crate::devices::{DeviceBox, IkeaOutlet, WakeOnLAN, AudioSetup, ContactSensor, KasaOutlet, AsOnOff};

// @TODO Configure more defaults

#[derive(Debug, Deserialize)]
pub struct Config {
    pub openid: OpenIDConfig,
    pub mqtt: MqttConfig,
    #[serde(default)]
    pub fullfillment: FullfillmentConfig,
    pub ntfy: Option<NtfyConfig>,
    pub presence: MqttDeviceConfig,
    pub light_sensor: LightSensorConfig,
    pub hue_bridge: Option<HueBridgeConfig>,
    #[serde(default)]
    pub devices: HashMap<String, Device>
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenIDConfig {
    pub base_url: String
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct InfoConfig {
    pub name: String,
    pub room: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MqttDeviceConfig {
    pub topic: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KettleConfig {
    pub timeout: Option<u64>, // Timeout in seconds
}

#[derive(Debug, Clone, Deserialize)]
pub struct PresenceDeviceConfig {
    #[serde(flatten)]
    pub mqtt: Option<MqttDeviceConfig>,
    // @TODO Maybe make this an option? That way if no timeout is set it will immediately turn the
    // device off again?
    pub timeout: u64 // Timeout in seconds
}

impl PresenceDeviceConfig {
    /// Set the mqtt topic to an appropriate value if it is not already set
    fn generate_topic(&mut self, class: &str, identifier: &str, config: &Config) {
        if self.mqtt.is_none() {
            // @TODO This is not perfect, if the topic is some/+/thing/# this will fail
            let offset = config.presence.topic.find('+').or(config.presence.topic.find('#')).unwrap();
            let topic = config.presence.topic[..offset].to_owned() + class + "/" + identifier;
            trace!("Setting presence mqtt topic: {topic}");
            self.mqtt = Some(MqttDeviceConfig { topic });
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
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
    KasaOutlet {
        ip: Ipv4Addr,
    },
    AudioSetup {
        #[serde(flatten)]
        mqtt: MqttDeviceConfig,
        mixer: Box::<Device>,
        speakers: Box::<Device>,
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
        let mut failure = false;
        let file = re.replace_all(&file, |caps: &Captures| {
            let key = caps.get(1).unwrap().as_str();
            debug!("Substituting '{key}' in config");
            match std::env::var(key) {
                Ok(value) => value,
                Err(_) => {
                    failure = true;
                    error!("Environment variable '{key}' is not set");
                    "".to_string()
                }
            }
        });

        if failure {
            return Err("Missing environment variables".into());
        }

        let config: Config = toml::from_str(&file)?;

        // Some extra config validation
        if !has_wildcards(&config.presence.topic) {
            return Err(format!("Invalid presence topic '{}', needs to contain a wildcard (+/#) in order to listen to presence devices", config.presence.topic).into());
        }

        // @TODO It would be nice it was possible to add validation to serde,
        // that way we can check that the provided mqtt topics are actually valid

        Ok(config)
    }
}

impl Device {
    #[async_recursion]
    pub async fn into(self, identifier: String, config: &Config, client: AsyncClient) -> DeviceBox {
        let device: DeviceBox = match self {
            Device::IkeaOutlet { info, mqtt, kettle } => {
                trace!(id = identifier, "IkeaOutlet [{} in {:?}]", info.name, info.room);
                Box::new(IkeaOutlet::new(identifier, info, mqtt, kettle, client).await)
            },
            Device::WakeOnLAN { info, mqtt, mac_address } => {
                trace!(id = identifier, "WakeOnLan [{} in {:?}]", info.name, info.room);
                Box::new(WakeOnLAN::new(identifier, info, mqtt, mac_address, client).await)
            },
            Device::KasaOutlet { ip } => {
                trace!(id = identifier, "KasaOutlet [{}]", identifier);
                Box::new(KasaOutlet::new(identifier, ip))
            }
            Device::AudioSetup { mqtt, mixer, speakers } => {
                trace!(id = identifier, "AudioSetup [{}]", identifier);
                // Create the child devices
                let mixer = (*mixer).into(identifier.clone() + ".mixer", config, client.clone()).await;
                let speakers = (*speakers).into(identifier.clone() + ".speakers", config, client.clone()).await;

                // The AudioSetup expects the children to be something that implements the OnOff trait
                // So let's convert the children and make sure OnOff is implemented
                let mixer = match AsOnOff::consume(mixer) {
                    Some(mixer) => mixer,
                    None => todo!("Handle this properly"),
                };
                let speakers = match AsOnOff::consume(speakers) {
                    Some(speakers) => speakers,
                    None => todo!("Handle this properly"),
                };

                Box::new(AudioSetup::new(identifier, mqtt, mixer, speakers, client).await)
            },
            Device::ContactSensor { mqtt, mut presence } => {
                trace!(id = identifier, "ContactSensor [{}]", identifier);
                if let Some(presence) = &mut presence {
                    presence.generate_topic("contact", &identifier, &config);
                }
                Box::new(ContactSensor::new(identifier, mqtt, presence, client).await)
            },
        };

        return device;
    }
}
