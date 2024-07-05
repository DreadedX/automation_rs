use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use google_home::errors::ErrorCode;
use google_home::traits::OnOff;
use rumqttc::{Publish, SubscribeFilter};
use tracing::{debug, error, trace, warn};

use super::{Device, LuaDeviceCreate};
use crate::config::MqttDeviceConfig;
use crate::event::OnMqtt;
use crate::messages::{RemoteAction, RemoteMessage};
use crate::mqtt::WrappedAsyncClient;
use crate::traits::Timeout;

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct HueGroupConfig {
    pub identifier: String,
    #[device_config(rename("ip"), with(|ip| SocketAddr::new(ip, 80)))]
    pub addr: SocketAddr,
    pub login: String,
    pub group_id: isize,
    pub timer_id: isize,
    pub scene_id: String,
    #[device_config(default)]
    pub remotes: Vec<MqttDeviceConfig>,
    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

#[derive(Debug, LuaDevice)]
pub struct HueGroup {
    config: HueGroupConfig,
}

// Couple of helper function to get the correct urls
#[async_trait]
impl LuaDeviceCreate for HueGroup {
    type Config = HueGroupConfig;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.identifier, "Setting up AudioSetup");

        if !config.remotes.is_empty() {
            config
                .client
                .subscribe_many(config.remotes.iter().map(|remote| SubscribeFilter {
                    path: remote.topic.clone(),
                    qos: rumqttc::QoS::AtLeastOnce,
                }))
                .await?;
        }

        Ok(Self { config })
    }
}

impl HueGroup {
    fn url_base(&self) -> String {
        format!("http://{}/api/{}", self.config.addr, self.config.login)
    }

    fn url_set_schedule(&self) -> String {
        format!("{}/schedules/{}", self.url_base(), self.config.timer_id)
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
impl OnMqtt for HueGroup {
    async fn on_mqtt(&mut self, message: Publish) {
        if !self
            .config
            .remotes
            .iter()
            .any(|remote| rumqttc::matches(&message.topic, &remote.topic))
        {
            return;
        }

        let action = match RemoteMessage::try_from(message) {
            Ok(message) => message.action(),
            Err(err) => {
                error!(
                    id = self.config.identifier,
                    "Failed to parse message: {err}"
                );
                return;
            }
        };

        debug!("Action: {action:#?}");

        match action {
            RemoteAction::On | RemoteAction::BrightnessMoveUp => self.set_on(true).await.unwrap(),
            RemoteAction::Off | RemoteAction::BrightnessMoveDown => {
                self.set_on(false).await.unwrap()
            }
            RemoteAction::BrightnessStop => { /* Ignore this action */ }
        };
    }
}

#[async_trait]
impl OnOff for HueGroup {
    async fn set_on(&mut self, on: bool) -> Result<(), ErrorCode> {
        // Abort any timer that is currently running
        self.stop_timeout().await.unwrap();

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
                    warn!(
                        id = self.config.identifier,
                        "Status code is not success: {status}"
                    );
                }
            }
            Err(err) => error!(id = self.config.identifier, "Error: {err}"),
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
                    warn!(
                        id = self.config.identifier,
                        "Status code is not success: {status}"
                    );
                }

                let on = match res.json::<message::Info>().await {
                    Ok(info) => info.any_on(),
                    Err(err) => {
                        error!(
                            id = self.config.identifier,
                            "Failed to parse message: {err}"
                        );
                        // TODO: Error code
                        return Ok(false);
                    }
                };

                return Ok(on);
            }
            Err(err) => error!(id = self.config.identifier, "Error: {err}"),
        }

        Ok(false)
    }
}

#[async_trait]
impl Timeout for HueGroup {
    async fn start_timeout(&mut self, timeout: Duration) -> Result<()> {
        // Abort any timer that is currently running
        self.stop_timeout().await?;

        // NOTE: This uses an existing timer, as we are unable to cancel it on the hub otherwise
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

    use serde::ser::SerializeStruct;
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

        // pub fn all_on(&self) -> bool {
        // 	self.state.all_on
        // }
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
