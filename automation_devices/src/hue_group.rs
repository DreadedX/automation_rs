use std::net::SocketAddr;

use anyhow::Result;
use async_trait::async_trait;
use automation_macro::LuaDeviceConfig;
use google_home::errors::ErrorCode;
use google_home::traits::OnOff;
use tracing::{error, trace, warn};

use super::{Device, LuaDeviceCreate};

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config {
    pub identifier: String,
    #[device_config(rename("ip"), with(|ip| SocketAddr::new(ip, 80)))]
    pub addr: SocketAddr,
    pub login: String,
    pub group_id: isize,
    pub scene_id: String,
}

#[derive(Debug, Clone)]
pub struct HueGroup {
    config: Config,
}

// Couple of helper function to get the correct urls
#[async_trait]
impl LuaDeviceCreate for HueGroup {
    type Config = Config;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.identifier, "Setting up AudioSetup");

        Ok(Self { config })
    }
}

impl HueGroup {
    fn url_base(&self) -> String {
        format!("http://{}/api/{}", self.config.addr, self.config.login)
    }

    fn url_set_action(&self) -> String {
        format!("{}/groups/{}/action", self.url_base(), self.config.group_id)
    }

    fn url_get_state(&self) -> String {
        format!("{}/groups/{}", self.url_base(), self.config.group_id)
    }
}

impl Device for HueGroup {
    fn get_id(&self) -> String {
        self.config.identifier.clone()
    }
}

#[async_trait]
impl OnOff for HueGroup {
    async fn set_on(&self, on: bool) -> Result<(), ErrorCode> {
        let message = if on {
            message::Action::scene(self.config.scene_id.clone())
        } else {
            message::Action::on(false)
        };

        let res = reqwest::Client::new()
            .put(self.url_set_action())
            .json(&message)
            .send()
            .await;

        match res {
            Ok(res) => {
                let status = res.status();
                if !status.is_success() {
                    warn!(id = self.get_id(), "Status code is not success: {status}");
                }
            }
            Err(err) => error!(id = self.get_id(), "Error: {err}"),
        }

        Ok(())
    }

    async fn on(&self) -> Result<bool, ErrorCode> {
        let res = reqwest::Client::new()
            .get(self.url_get_state())
            .send()
            .await;

        match res {
            Ok(res) => {
                let status = res.status();
                if !status.is_success() {
                    warn!(id = self.get_id(), "Status code is not success: {status}");
                }

                let on = match res.json::<message::Info>().await {
                    Ok(info) => info.any_on(),
                    Err(err) => {
                        error!(id = self.get_id(), "Failed to parse message: {err}");
                        // TODO: Error code
                        return Ok(false);
                    }
                };

                return Ok(on);
            }
            Err(err) => error!(id = self.get_id(), "Error: {err}"),
        }

        Ok(false)
    }
}

mod message {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Action {
        #[serde(skip_serializing_if = "Option::is_none")]
        on: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        scene: Option<String>,
    }

    impl Action {
        pub fn on(on: bool) -> Self {
            Self {
                on: Some(on),
                scene: None,
            }
        }

        pub fn scene(scene: String) -> Self {
            Self {
                on: None,
                scene: Some(scene),
            }
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct State {
        all_on: bool,
        any_on: bool,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Info {
        state: State,
    }

    impl Info {
        pub fn any_on(&self) -> bool {
            self.state.any_on
        }
    }
}
