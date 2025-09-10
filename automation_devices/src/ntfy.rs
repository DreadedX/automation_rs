use std::collections::HashMap;
use std::convert::Infallible;

use async_trait::async_trait;
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_macro::{Device, LuaDeviceConfig};
use mlua::LuaSerdeExt;
use serde::{Deserialize, Serialize};
use serde_repr::*;
use tracing::{error, trace, warn};

#[derive(Debug, Serialize_repr, Deserialize, Clone, Copy)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Min = 1,
    Low,
    Default,
    High,
    Max,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum ActionType {
    Broadcast {
        #[serde(skip_serializing_if = "HashMap::is_empty")]
        extras: HashMap<String, String>,
    },
    // View,
    // Http
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Action {
    #[serde(flatten)]
    pub action: ActionType,
    pub label: String,
    pub clear: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct NotificationFinal {
    topic: String,
    #[serde(flatten)]
    inner: Notification,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct Notification {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Default::default")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<Priority>,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Default::default")]
    actions: Vec<Action>,
}

impl Notification {
    pub fn new() -> Self {
        Self {
            title: None,
            message: None,
            tags: Vec::new(),
            priority: None,
            actions: Vec::new(),
        }
    }

    fn finalize(self, topic: &str) -> NotificationFinal {
        NotificationFinal {
            topic: topic.into(),
            inner: self,
        }
    }
}

impl Default for Notification {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config {
    #[device_config(default("https://ntfy.sh".into()))]
    pub url: String,
    pub topic: String,
}

#[derive(Debug, Clone, Device)]
#[device(add_methods(Self::add_methods))]
pub struct Ntfy {
    config: Config,
}
crate::register_device!(Ntfy);

impl Ntfy {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method(
            "send_notification",
            async |lua, this, notification: mlua::Value| {
                let notification: Notification = lua.from_value(notification)?;

                this.send(notification).await;

                Ok(())
            },
        );
    }
}

#[async_trait]
impl LuaDeviceCreate for Ntfy {
    type Config = Config;
    type Error = Infallible;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = "ntfy", "Setting up Ntfy");
        Ok(Self { config })
    }
}

impl Device for Ntfy {
    fn get_id(&self) -> String {
        "ntfy".to_string()
    }
}

impl Ntfy {
    async fn send(&self, notification: Notification) {
        let notification = notification.finalize(&self.config.topic);

        // Create the request
        let res = reqwest::Client::new()
            .post(self.config.url.clone())
            .json(&notification)
            .send()
            .await;

        if let Err(err) = res {
            error!("Something went wrong while sending the notification: {err}");
        } else if let Ok(res) = res {
            let status = res.status();
            if !status.is_success() {
                warn!("Received status {status} when sending notification");
            }
        }
    }
}
