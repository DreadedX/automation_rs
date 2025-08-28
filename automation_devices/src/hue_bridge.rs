use std::convert::Infallible;
use std::net::SocketAddr;

use async_trait::async_trait;
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::event::{OnDarkness, OnPresence};
use automation_macro::{LuaDeviceConfig, impl_device};
use serde::{Deserialize, Serialize};
use tracing::{error, trace, warn};

#[derive(Debug)]
pub enum Flag {
    Presence,
    Darkness,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlagIDs {
    presence: isize,
    darkness: isize,
}

#[derive(Debug, LuaDeviceConfig, Clone)]
pub struct Config {
    pub identifier: String,
    #[device_config(rename("ip"), with(|ip| SocketAddr::new(ip, 80)))]
    pub addr: SocketAddr,
    pub login: String,
    pub flags: FlagIDs,
}

#[derive(Debug, Clone)]
pub struct HueBridge {
    config: Config,
}
impl_device!(HueBridge);

#[derive(Debug, Serialize)]
struct FlagMessage {
    flag: bool,
}

#[async_trait]
impl LuaDeviceCreate for HueBridge {
    type Config = Config;
    type Error = Infallible;

    async fn create(config: Self::Config) -> Result<Self, Infallible> {
        trace!(id = config.identifier, "Setting up HueBridge");
        Ok(Self { config })
    }
}

impl HueBridge {
    pub async fn set_flag(&self, flag: Flag, value: bool) {
        let flag_id = match flag {
            Flag::Presence => self.config.flags.presence,
            Flag::Darkness => self.config.flags.darkness,
        };

        let url = format!(
            "http://{}/api/{}/sensors/{flag_id}/state",
            self.config.addr, self.config.login
        );

        trace!(?flag, flag_id, value, "Sending request to change flag");
        let res = reqwest::Client::new()
            .put(url)
            .json(&FlagMessage { flag: value })
            .send()
            .await;

        match res {
            Ok(res) => {
                let status = res.status();
                if !status.is_success() {
                    warn!(flag_id, "Status code is not success: {status}");
                }
            }
            Err(err) => {
                error!(flag_id, "Error: {err}");
            }
        }
    }
}

impl Device for HueBridge {
    fn get_id(&self) -> String {
        self.config.identifier.clone()
    }
}

#[async_trait]
impl OnPresence for HueBridge {
    async fn on_presence(&self, presence: bool) {
        trace!("Bridging presence to hue");
        self.set_flag(Flag::Presence, presence).await;
    }
}

#[async_trait]
impl OnDarkness for HueBridge {
    async fn on_darkness(&self, dark: bool) {
        trace!("Bridging darkness to hue");
        self.set_flag(Flag::Darkness, dark).await;
    }
}
