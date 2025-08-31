use std::collections::HashMap;
use std::convert::Infallible;

use async_trait::async_trait;
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::event::{self, EventChannel};
use automation_lib::lua::traits::AddAdditionalMethods;
use automation_macro::{LuaDevice, LuaDeviceConfig};
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

    pub fn set_title(mut self, title: &str) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn set_message(mut self, message: &str) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn add_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn set_priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn add_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
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
    #[device_config(rename("event_channel"), from_lua, with(|ec: EventChannel| ec.get_tx()))]
    pub tx: event::Sender,
}

#[derive(Debug, Clone, LuaDevice)]
#[traits(AddAdditionalMethods)]
pub struct Ntfy {
    config: Config,
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

impl AddAdditionalMethods for Ntfy {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M)
    where
        Self: Sized + 'static,
    {
        methods.add_async_method(
            "send_notification",
            |lua, this, notification: mlua::Value| async move {
                let notification: Notification = lua.from_value(notification)?;

                this.send(notification).await;

                Ok(())
            },
        );
    }
}
