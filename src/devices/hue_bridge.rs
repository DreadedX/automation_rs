use std::net::{Ipv4Addr, SocketAddr};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{error, trace, warn};

use crate::{
    device_manager::{ConfigExternal, DeviceConfig},
    devices::Device,
    error::DeviceConfigError,
    event::OnDarkness,
    event::OnPresence,
};

#[derive(Debug)]
pub enum Flag {
    Presence,
    Darkness,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlagIDs {
    pub presence: isize,
    pub darkness: isize,
}

#[derive(Debug, Deserialize)]
pub struct HueBridgeConfig {
    pub ip: Ipv4Addr,
    pub login: String,
    pub flags: FlagIDs,
}

#[async_trait]
impl DeviceConfig for HueBridgeConfig {
    async fn create(
        self,
        identifier: &str,
        _ext: &ConfigExternal,
    ) -> Result<Box<dyn Device>, DeviceConfigError> {
        let device = HueBridge {
            identifier: identifier.into(),
            addr: (self.ip, 80).into(),
            login: self.login,
            flag_ids: self.flags,
        };

        Ok(Box::new(device))
    }
}

#[derive(Debug)]
struct HueBridge {
    identifier: String,
    addr: SocketAddr,
    login: String,
    flag_ids: FlagIDs,
}

#[derive(Debug, Serialize)]
struct FlagMessage {
    flag: bool,
}

impl HueBridge {
    pub async fn set_flag(&self, flag: Flag, value: bool) {
        let flag_id = match flag {
            Flag::Presence => self.flag_ids.presence,
            Flag::Darkness => self.flag_ids.darkness,
        };

        let url = format!(
            "http://{}/api/{}/sensors/{flag_id}/state",
            self.addr, self.login
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
    fn get_id(&self) -> &str {
        &self.identifier
    }
}

#[async_trait]
impl OnPresence for HueBridge {
    async fn on_presence(&mut self, presence: bool) {
        trace!("Bridging presence to hue");
        self.set_flag(Flag::Presence, presence).await;
    }
}

#[async_trait]
impl OnDarkness for HueBridge {
    async fn on_darkness(&mut self, dark: bool) {
        trace!("Bridging darkness to hue");
        self.set_flag(Flag::Darkness, dark).await;
    }
}
