use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};

use lua_typed::Typed;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Setup {
    #[serde(default = "default_entrypoint")]
    pub entrypoint: String,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub secrets: HashMap<String, String>,
}

fn default_entrypoint() -> String {
    "./config.lua".into()
}

#[derive(Debug, Deserialize, Typed)]
pub struct FulfillmentConfig {
    pub openid_url: String,
    #[serde(default = "default_fulfillment_ip")]
    #[typed(default)]
    pub ip: Ipv4Addr,
    #[serde(default = "default_fulfillment_port")]
    #[typed(default)]
    pub port: u16,
}

#[derive(Debug, Deserialize, Typed)]
pub struct Config {
    pub fulfillment: FulfillmentConfig,
}

impl From<FulfillmentConfig> for SocketAddr {
    fn from(fulfillment: FulfillmentConfig) -> Self {
        (fulfillment.ip, fulfillment.port).into()
    }
}
fn default_fulfillment_ip() -> Ipv4Addr {
    [0, 0, 0, 0].into()
}

fn default_fulfillment_port() -> u16 {
    7878
}
