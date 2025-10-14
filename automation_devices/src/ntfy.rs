use std::collections::HashMap;
use std::convert::Infallible;

use async_trait::async_trait;
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::lua::traits::PartialUserData;
use automation_macro::{Device, LuaDeviceConfig};
use lua_typed::Typed;
use mlua::LuaSerdeExt;
use serde::{Deserialize, Serialize};
use serde_repr::*;
use tracing::{error, trace, warn};

#[derive(Debug, Serialize_repr, Deserialize, Clone, Copy, Typed)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
#[typed(rename_all = "snake_case")]
pub enum Priority {
    Min = 1,
    Low,
    Default,
    High,
    Max,
}
crate::register_type!(Priority);

#[derive(Debug, Serialize, Deserialize, Clone, Typed)]
#[serde(rename_all = "snake_case", tag = "action")]
#[typed(rename_all = "snake_case", tag = "action")]
pub enum ActionType {
    Broadcast {
        #[serde(skip_serializing_if = "HashMap::is_empty")]
        #[serde(default)]
        #[typed(default)]
        extras: HashMap<String, String>,
    },
    // View,
    // Http
}

#[derive(Debug, Serialize, Deserialize, Clone, Typed)]
pub struct Action {
    #[serde(flatten)]
    #[typed(flatten)]
    pub action: ActionType,
    pub label: String,
    pub clear: Option<bool>,
}
crate::register_type!(Action);

#[derive(Serialize, Deserialize, Typed)]
struct NotificationFinal {
    topic: String,
    #[serde(flatten)]
    #[typed(flatten)]
    inner: Notification,
}

#[derive(Debug, Serialize, Clone, Deserialize, Typed)]
pub struct Notification {
    title: String,
    message: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Default::default")]
    #[typed(default)]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<Priority>,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Default::default")]
    #[typed(default)]
    actions: Vec<Action>,
}
crate::register_type!(Notification);

impl Notification {
    fn finalize(self, topic: &str) -> NotificationFinal {
        NotificationFinal {
            topic: topic.into(),
            inner: self,
        }
    }
}

#[derive(Debug, Clone, LuaDeviceConfig, Typed)]
#[typed(as = "NtfyConfig")]
pub struct Config {
    #[device_config(default("https://ntfy.sh".into()))]
    #[typed(default)]
    pub url: String,
    pub topic: String,
}
crate::register_type!(Config);

#[derive(Debug, Clone, Device)]
#[device(extra_user_data = SendNotification)]
pub struct Ntfy {
    config: Config,
}
crate::register_device!(Ntfy);

struct SendNotification;
impl PartialUserData<Ntfy> for SendNotification {
    fn add_methods<M: mlua::UserDataMethods<Ntfy>>(methods: &mut M) {
        methods.add_async_method(
            "send_notification",
            async |lua, this, notification: mlua::Value| {
                let notification: Notification = lua.from_value(notification)?;

                this.send(notification).await;

                Ok(())
            },
        );
    }

    fn definitions() -> Option<String> {
        Some(format!(
            "---@async\n---@param notification {}\nfunction {}:send_notification(notification) end\n",
            <Notification as Typed>::type_name(),
            <Ntfy as Typed>::type_name(),
        ))
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
