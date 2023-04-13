use std::net::{Ipv4Addr, SocketAddr};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{error, trace, warn};

use crate::{
    light_sensor::{self, OnDarkness},
    presence::{self, OnPresence},
};

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
struct HueBridge {
    addr: SocketAddr,
    login: String,
    flags: FlagIDs,
}

#[derive(Debug, Serialize)]
struct FlagMessage {
    flag: bool,
}

impl HueBridge {
    pub fn new(config: HueBridgeConfig) -> Self {
        Self {
            addr: (config.ip, 80).into(),
            login: config.login,
            flags: config.flags,
        }
    }

    pub async fn set_flag(&self, flag: Flag, value: bool) {
        let flag = match flag {
            Flag::Presence => self.flags.presence,
            Flag::Darkness => self.flags.darkness,
        };

        let url = format!(
            "http://{}/api/{}/sensors/{flag}/state",
            self.addr, self.login
        );
        let res = reqwest::Client::new()
            .put(url)
            .json(&FlagMessage { flag: value })
            .send()
            .await;

        match res {
            Ok(res) => {
                let status = res.status();
                if !status.is_success() {
                    warn!(flag, "Status code is not success: {status}");
                }
            }
            Err(err) => {
                error!(flag, "Error: {err}");
            }
        }
    }
}

pub fn start(
    mut presence_rx: presence::Receiver,
    mut light_sensor_rx: light_sensor::Receiver,
    config: HueBridgeConfig,
) {
    let mut hue_bridge = HueBridge::new(config);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                res = presence_rx.changed() => {
                    if res.is_err() {
                        break;
                    }

                    let presence = *presence_rx.borrow();
                    hue_bridge.on_presence(presence).await;
                }
                res = light_sensor_rx.changed() => {
                    if res.is_err() {
                        break;
                    }

                    let darkness = *light_sensor_rx.borrow();
                    hue_bridge.on_darkness(darkness).await;
                }
            }
        }

        unreachable!("Did not expect this");
    });
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
