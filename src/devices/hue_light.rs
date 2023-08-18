use std::{
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use google_home::{errors::ErrorCode, traits::OnOff};
use serde::Deserialize;
use tracing::{error, warn};

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

// Couple of helper function to get the correct urls
impl HueLight {
    fn url_base(&self) -> String {
        format!("http://{}/api/{}", self.addr, self.login)
    }

    fn url_set_schedule(&self) -> String {
        format!("{}/schedules/{}", self.url_base(), self.timer_id)
    }

    fn url_set_state(&self) -> String {
        format!("{}/lights/{}/state", self.url_base(), self.light_id)
    }

    fn url_get_state(&self) -> String {
        format!("{}/lights/{}", self.url_base(), self.light_id)
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
        self.stop_timeout().await.unwrap();

        let message = message::State::new(on);
        let res = reqwest::Client::new()
            .put(self.url_set_state())
            .json(&message)
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
        let res = reqwest::Client::new()
            .get(self.url_get_state())
            .send()
            .await;

        match res {
            Ok(res) => {
                let status = res.status();
                if !status.is_success() {
                    warn!(id = self.identifier, "Status code is not success: {status}");
                }

                let on = match res.json::<message::Info>().await {
                    Ok(info) => info.is_on(),
                    Err(err) => {
                        error!(id = self.identifier, "Failed to parse message: {err}");
                        // TODO: Error code
                        return Ok(false);
                    }
                };

                return Ok(on);
            }
            Err(err) => error!(id = self.identifier, "Error: {err}"),
        }

        Ok(false)
    }
}

#[async_trait]
impl Timeout for HueLight {
    async fn start_timeout(&mut self, timeout: Duration) -> Result<()> {
        // Abort any timer that is currently running
        self.stop_timeout().await?;

        let message = message::Timeout::new(Some(timeout));
        let res = reqwest::Client::new()
            .put(self.url_set_schedule())
            .json(&message)
            .send()
            .await
            .context("Failed to start timeout")?;

        let status = res.status();
        if !status.is_success() {
            return Err(anyhow!(
                "Hue bridge returned unsuccessful status '{status}'"
            ));
        }

        Ok(())
    }

    async fn stop_timeout(&mut self) -> Result<()> {
        let message = message::Timeout::new(None);
        let res = reqwest::Client::new()
            .put(self.url_set_schedule())
            .json(&message)
            .send()
            .await
            .context("Failed to stop timeout")?;

        let status = res.status();
        if !status.is_success() {
            return Err(anyhow!(
                "Hue bridge returned unsuccessful status '{status}'"
            ));
        }

        Ok(())
    }
}

mod message {
    use std::time::Duration;

    use serde::{ser::SerializeStruct, Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct State {
        on: bool,
    }

    impl State {
        pub fn new(on: bool) -> Self {
            Self { on }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Info {
        state: State,
    }

    impl Info {
        pub fn is_on(&self) -> bool {
            self.state.on
        }
    }

    #[derive(Debug)]
    pub struct Timeout {
        timeout: Option<Duration>,
    }

    impl Timeout {
        pub fn new(timeout: Option<Duration>) -> Self {
            Self { timeout }
        }
    }

    impl Serialize for Timeout {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let len = if self.timeout.is_some() { 2 } else { 1 };
            let mut state = serializer.serialize_struct("TimerMessage", len)?;
            if self.timeout.is_some() {
                state.serialize_field("status", "enabled")?;
            } else {
                state.serialize_field("status", "disabled")?;
            }

            if let Some(timeout) = self.timeout {
                let seconds = timeout.as_secs() % 60;
                let minutes = (timeout.as_secs() / 60) % 60;
                let hours = timeout.as_secs() / 3600;

                let time = format!("PT{hours:<02}:{minutes:<02}:{seconds:<02}");
                state.serialize_field("localtime", &time)?;
            };

            state.end()
        }
    }
}
