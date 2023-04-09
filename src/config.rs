use std::{fs, net::{Ipv4Addr, SocketAddr}, collections::HashMap};

use async_recursion::async_recursion;
use regex::{Regex, Captures};
use tracing::{debug, trace};
use rumqttc::{AsyncClient, has_wildcards};
use serde::Deserialize;
use eui48::MacAddress;

use crate::{devices::{DeviceBox, IkeaOutlet, WakeOnLAN, AudioSetup, ContactSensor, KasaOutlet, self}, error::{MissingEnv, MissingWildcard, ConfigParseError, DeviceCreationError}};

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
    pub debug_bridge: Option<DebugBridgeConfig>,
    #[serde(default)]
    pub devices: HashMap<String, Device>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenIDConfig {
    pub base_url: String
}

#[derive(Debug, Clone, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub client_name: String,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub tls: bool,
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
    [0, 0, 0, 0].into()
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

#[derive(Debug, Clone, Deserialize)]
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
pub struct DebugBridgeConfig {
    pub topic: String,
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub enum OutletType {
    Outlet,
    Kettle,
    Charger,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KettleConfig {
    pub timeout: Option<u64>, // Timeout in seconds
}

#[derive(Debug, Clone, Deserialize)]
pub struct PresenceDeviceConfig {
    #[serde(flatten)]
    pub mqtt: Option<MqttDeviceConfig>,
    // TODO: Maybe make this an option? That way if no timeout is set it will immediately turn the
    // device off again?
    pub timeout: u64 // Timeout in seconds
}

impl PresenceDeviceConfig {
    /// Set the mqtt topic to an appropriate value if it is not already set
    fn generate_topic(mut self, class: &str, identifier: &str, config: &Config) -> Result<PresenceDeviceConfig, MissingWildcard> {
        if self.mqtt.is_none() {
            if !has_wildcards(&config.presence.topic) {
                return Err(MissingWildcard::new(&config.presence.topic).into());
            }

            // TODO: This is not perfect, if the topic is some/+/thing/# this will fail
            let offset = config.presence.topic.find('+').or(config.presence.topic.find('#')).unwrap();
            let topic = format!("{}/{class}/{identifier}", &config.presence.topic[..offset-1]);
            trace!("Setting presence mqtt topic: {topic}");
            self.mqtt = Some(MqttDeviceConfig { topic });
        }

        Ok(self)
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
        #[serde(default = "default_outlet_type")]
        outlet_type: OutletType,
        timeout: Option<u64>, // Timeout in seconds
    },
    WakeOnLAN {
        #[serde(flatten)]
        info: InfoConfig,
        #[serde(flatten)]
        mqtt: MqttDeviceConfig,
        mac_address: MacAddress,
        #[serde(default = "default_broadcast_ip")]
        broadcast_ip: Ipv4Addr,
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

fn default_outlet_type() -> OutletType {
    OutletType::Outlet
}

fn default_broadcast_ip() -> Ipv4Addr {
    Ipv4Addr::new(255, 255, 255, 255)
}

impl Config {
    pub fn parse_file(filename: &str) -> Result<Self, ConfigParseError> {
        debug!("Loading config: {filename}");
        let file = fs::read_to_string(filename)?;

        // Substitute in environment variables
        let re = Regex::new(r"\$\{(.*)\}").unwrap();
        let mut missing = MissingEnv::new();
        let file = re.replace_all(&file, |caps: &Captures| {
            let key = caps.get(1).unwrap().as_str();
            debug!("Substituting '{key}' in config");
            match std::env::var(key) {
                Ok(value) => value,
                Err(_) => {
                    missing.add_missing(key);
                    "".to_string()
                }
            }
        });

        missing.has_missing()?;

        let config: Config = toml::from_str(&file)?;

        Ok(config)
    }
}

// Quick helper function to box up the devices,
// passing in Box::new would be ideal, however the return type is incorrect
// Maybe there is a better way to solve this?
fn device_box<T: devices::Device + 'static>(device: T) -> DeviceBox {
    let a: DeviceBox = Box::new(device);
    a
}

impl Device {
    #[async_recursion]
    pub async fn create(self, identifier: &str, config: &Config, client: AsyncClient) -> Result<DeviceBox, DeviceCreationError> {
        let device = match self {
            Device::IkeaOutlet { info, mqtt, outlet_type, timeout } => {
                trace!(id = identifier, "IkeaOutlet [{} in {:?}]", info.name, info.room);
                IkeaOutlet::build(&identifier, info, mqtt, outlet_type, timeout, client).await
                    .map(device_box)?
            },
            Device::WakeOnLAN { info, mqtt, mac_address, broadcast_ip } => {
                trace!(id = identifier, "WakeOnLan [{} in {:?}]", info.name, info.room);
                WakeOnLAN::build(&identifier, info, mqtt, mac_address, broadcast_ip, client).await
                    .map(device_box)?
            },
            Device::KasaOutlet { ip } => {
                trace!(id = identifier, "KasaOutlet [{}]", identifier);
                device_box(KasaOutlet::new(&identifier, ip))
            }
            Device::AudioSetup { mqtt, mixer, speakers } => {
                trace!(id = identifier, "AudioSetup [{}]", identifier);
                // Create the child devices
                let mixer_id = format!("{}.mixer", identifier);
                let mixer = (*mixer).create(&mixer_id, config, client.clone()).await?;
                let speakers_id = format!("{}.speakers", identifier);
                let speakers = (*speakers).create(&speakers_id, config, client.clone()).await?;

                AudioSetup::build(&identifier, mqtt, mixer, speakers, client).await
                    .map(device_box)?
            },
            Device::ContactSensor { mqtt, presence } => {
                trace!(id = identifier, "ContactSensor [{}]", identifier);
                let presence = presence
                    .map(|p| p.generate_topic("contact", &identifier, &config))
                    .transpose()?;

                ContactSensor::build(&identifier, mqtt, presence, client).await
                    .map(device_box)?
            },
        };

        Ok(device)
    }
}
