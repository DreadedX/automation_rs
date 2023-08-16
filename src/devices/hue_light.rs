use std::{
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use async_trait::async_trait;
use google_home::{errors::ErrorCode, traits::OnOff};
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, error, warn};

use crate::{
    device_manager::{ConfigExternal, DeviceConfig},
    error::DeviceConfigError,
    traits::Timeout,
};

use super::Device;

#[derive(Debug, Clone, Deserialize)]
pub struct HueLightConfig {
    pub ip: Ipv4Addr,
    pub login: String,
    pub light_id: isize,
    pub timer_id: isize,
}

#[async_trait]
impl DeviceConfig for HueLightConfig {
    async fn create(
        self,
        identifier: &str,
        _ext: &ConfigExternal,
    ) -> Result<Box<dyn Device>, DeviceConfigError> {
        let device = HueLight {
            identifier: identifier.into(),
            addr: (self.ip, 80).into(),
            login: self.login,
            light_id: self.light_id,
            timer_id: self.timer_id,
        };

        Ok(Box::new(device))
    }
}

#[derive(Debug)]
struct HueLight {
    pub identifier: String,
    pub addr: SocketAddr,
    pub login: String,
    pub light_id: isize,
    pub timer_id: isize,
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
                    warn!(id = self.identifier, "Status code is not success: {status}");
                }
            }
            Err(err) => error!(id = self.identifier, "Error: {err}"),
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
                    warn!(id = self.identifier, "Status code is not success: {status}");
                }

                let v: Value = serde_json::from_slice(res.bytes().await.unwrap().as_ref()).unwrap();
                // TODO: This is not very nice
                return Ok(v["state"]["on"].as_bool().unwrap());
            }
            Err(err) => error!(id = self.identifier, "Error: {err}"),
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

        debug!(id = self.identifier, "Starting timeout ({time})...");

        let res = reqwest::Client::new()
            .put(url)
            .body(format!(r#"{{"status": "enabled", "localtime": "{time}"}}"#))
            .send()
            .await;

        match res {
            Ok(res) => {
                let status = res.status();
                if !status.is_success() {
                    warn!(id = self.identifier, "Status code is not success: {status}");
                }
            }
            Err(err) => error!(id = self.identifier, "Error: {err}"),
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
                    warn!(id = self.identifier, "Status code is not success: {status}");
                }
            }
            Err(err) => error!(id = self.identifier, "Error: {err}"),
        }
    }
}
