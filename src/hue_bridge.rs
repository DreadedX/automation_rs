use std::net::SocketAddr;

use pollster::FutureExt;
use serde::Serialize;
use tracing::{warn, error, trace};

use crate::{config::{HueBridgeConfig, Flags}, presence::OnPresence, light_sensor::OnDarkness};

pub enum Flag {
    Presence,
    Darkness,
}

pub struct HueBridge {
    addr: SocketAddr,
    login: String,
    flags: Flags,
}

#[derive(Debug, Serialize)]
struct FlagMessage {
    flag: bool
}

impl HueBridge {
    pub fn new(config: HueBridgeConfig) -> Self {
        Self {
            addr: (config.ip, 80).into(),
            login: config.login,
            flags: config.flags,
        }
    }

    pub fn set_flag(&self, flag: Flag, value: bool) {
        let flag = match flag {
            Flag::Presence => self.flags.presence,
            Flag::Darkness => self.flags.darkness,
        };

        let url = format!("http://{}/api/{}/sensors/{flag}/state", self.addr, self.login);
        let res = reqwest::Client::new()
            .put(url)
            .json(&FlagMessage { flag: value })
            .send()
            .block_on();

        match res {
            Ok(res) => {
                let status = res.status();
                if !status.is_success() {
                    warn!(flag, "Status code is not success: {status}");
                }
            },
            Err(err) => {
                error!(flag, "Error: {err}");
            }
        }
    }
}

impl OnPresence for HueBridge {
    fn on_presence(&mut self, presence: bool) {
        trace!("Bridging presence to hue");
        self.set_flag(Flag::Presence, presence);
    }
}

impl OnDarkness for HueBridge {
    fn on_darkness(&mut self, dark: bool) {
        trace!("Bridging darkness to hue");
        self.set_flag(Flag::Darkness, dark);
    }
}
