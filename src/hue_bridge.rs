use std::net::{Ipv4Addr, SocketAddr};

use serde::{Deserialize, Serialize};
use tracing::{error, trace, warn};

use crate::event::{Event, EventChannel};

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
    flag_ids: FlagIDs,
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
            flag_ids: config.flags,
        }
    }

    pub async fn set_flag(&self, flag: Flag, value: bool) {
        let flag_id = match flag {
            Flag::Presence => self.flag_ids.presence,
            Flag::Darkness => self.flag_ids.darkness,
        };

        let url = format!(
            "http://{}/api/{}/sensors/{flag_id}/state",
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
                    warn!(flag_id, "Status code is not success: {status}");
                }
            }
            Err(err) => {
                error!(flag_id, "Error: {err}");
            }
        }
    }
}

pub fn start(config: HueBridgeConfig, event_channel: &EventChannel) {
    let hue_bridge = HueBridge::new(config);

    let mut rx = event_channel.get_rx();

    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(Event::Presence(presence)) => {
                    trace!("Bridging presence to hue");
                    hue_bridge.set_flag(Flag::Presence, presence).await;
                }
                Ok(Event::Darkness(dark)) => {
                    trace!("Bridging darkness to hue");
                    hue_bridge.set_flag(Flag::Darkness, dark).await;
                }
                Ok(_) => {}
                Err(_) => todo!("Handle errors with the event channel properly"),
            }
        }
    });
}
