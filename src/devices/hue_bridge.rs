use std::{
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use async_trait::async_trait;
use google_home::{errors::ErrorCode, traits::OnOff};
use rumqttc::AsyncClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error, trace, warn};

use crate::{
    config::CreateDevice, device_manager::DeviceManager, devices::Device, error::CreateDeviceError,
    event::EventChannel, event::OnDarkness, event::OnPresence, traits::Timeout,
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

#[derive(Debug)]
pub struct HueBridge {
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

    pub fn new(config: HueBridgeConfig) -> Self {
        Self {
            addr: (config.ip, 80).into(),
            login: config.login,
            flag_ids: config.flags,
        }
    }
}

impl Device for HueBridge {
    fn get_id(&self) -> &str {
        "hue_bridge"
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

#[derive(Debug, Clone, Deserialize)]
pub struct HueLightConfig {
    pub ip: Ipv4Addr,
    pub login: String,
    pub light_id: isize,
    pub timer_id: isize,
}

#[derive(Debug)]
pub struct HueLight {
    pub identifier: String,
    pub addr: SocketAddr,
    pub login: String,
    pub light_id: isize,
    pub timer_id: isize,
}

#[async_trait]
impl CreateDevice for HueLight {
    type Config = HueLightConfig;

    async fn create(
        identifier: &str,
        config: Self::Config,
        _event_channel: &EventChannel,
        _client: &AsyncClient,
        _presence_topic: &str,
        _devices: &DeviceManager,
    ) -> Result<Self, CreateDeviceError> {
        Ok(Self {
            identifier: identifier.to_owned(),
            addr: (config.ip, 80).into(),
            login: config.login,
            light_id: config.light_id,
            timer_id: config.timer_id,
        })
    }
}

impl Device for HueLight {
    fn get_id(&self) -> &str {
        &self.identifier
    }
}

#[async_trait]
impl OnOff for HueLight {
    async fn set_on(&mut self, on: bool) -> Result<(), ErrorCode> {
        // Abort any timer that is currently running
        self.stop_timeout().await;

        let url = format!(
            "http://{}/api/{}/lights/{}/state",
            self.addr, self.login, self.light_id
        );

        let res = reqwest::Client::new()
            .put(url)
            .body(format!(r#"{{"on": {}}}"#, on))
            .send()
            .await;

        match res {
            Ok(res) => {
                let status = res.status();
                if !status.is_success() {
                    warn!(self.identifier, "Status code is not success: {status}");
                }
            }
            Err(err) => error!(self.identifier, "Error: {err}"),
        }

        Ok(())
    }

    async fn is_on(&self) -> Result<bool, ErrorCode> {
        let url = format!(
            "http://{}/api/{}/lights/{}",
            self.addr, self.login, self.light_id
        );

        let res = reqwest::Client::new().get(url).send().await;

        match res {
            Ok(res) => {
                let status = res.status();
                if !status.is_success() {
                    warn!(self.identifier, "Status code is not success: {status}");
                }

                let v: Value = serde_json::from_slice(res.bytes().await.unwrap().as_ref()).unwrap();
                // TODO: This is not very nice
                return Ok(v["state"]["on"].as_bool().unwrap());
            }
            Err(err) => error!(self.identifier, "Error: {err}"),
        }

        Ok(false)
    }
}

#[async_trait]
impl Timeout for HueLight {
    async fn start_timeout(&mut self, timeout: Duration) {
        // Abort any timer that is currently running
        self.stop_timeout().await;

        let url = format!(
            "http://{}/api/{}/schedules/{}",
            self.addr, self.login, self.timer_id
        );

        let seconds = timeout.as_secs() % 60;
        let minutes = (timeout.as_secs() / 60) % 60;
        let hours = timeout.as_secs() / 3600;
        let time = format!("PT{hours:<02}:{minutes:<02}:{seconds:<02}");

        debug!(self.identifier, "Starting timeout ({time})...");

        let res = reqwest::Client::new()
            .put(url)
            .body(format!(r#"{{"status": "enabled", "localtime": "{time}"}}"#))
            .send()
            .await;

        match res {
            Ok(res) => {
                let status = res.status();
                if !status.is_success() {
                    warn!(self.identifier, "Status code is not success: {status}");
                }
            }
            Err(err) => error!(self.identifier, "Error: {err}"),
        }
    }

    async fn stop_timeout(&mut self) {
        let url = format!(
            "http://{}/api/{}/schedules/{}",
            self.addr, self.login, self.timer_id
        );

        let res = reqwest::Client::new()
            .put(url)
            .body(format!(r#"{{"status": "disabled"}}"#))
            .send()
            .await;

        match res {
            Ok(res) => {
                let status = res.status();
                if !status.is_success() {
                    warn!(self.identifier, "Status code is not success: {status}");
                }
            }
            Err(err) => error!(self.identifier, "Error: {err}"),
        }
    }
}
