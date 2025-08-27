use std::collections::HashMap;
use std::convert::Infallible;
use std::ops::Deref;

use async_trait::async_trait;
use automation_cast::Cast;
use automation_macro::LuaDeviceConfig;
use serde::Serialize;
use serde_repr::*;
use tracing::{error, trace, warn};

use crate::device::{Device, LuaDeviceCreate, impl_device};
use crate::event::{self, Event, EventChannel, OnNotification, OnPresence};

#[derive(Debug, Serialize_repr, Clone, Copy)]
#[repr(u8)]
pub enum Priority {
    Min = 1,
    Low,
    Default,
    High,
    Max,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum ActionType {
    Broadcast {
        #[serde(skip_serializing_if = "HashMap::is_empty")]
        extras: HashMap<String, String>,
    },
    // View,
    // Http
}

#[derive(Debug, Serialize, Clone)]
pub struct Action {
    #[serde(flatten)]
    pub action: ActionType,
    pub label: String,
    pub clear: Option<bool>,
}

#[derive(Serialize)]
struct NotificationFinal {
    topic: String,
    #[serde(flatten)]
    inner: Notification,
}

#[derive(Debug, Serialize, Clone)]
pub struct Notification {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<Priority>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
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

#[derive(Debug, Clone)]
pub struct Ntfy {
    config: Config,
}

impl_device!(Ntfy);

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

#[async_trait]
impl OnPresence for Ntfy {
    async fn on_presence(&self, presence: bool) {
        // Setup extras for the broadcast
        let extras = HashMap::from([
            ("cmd".into(), "presence".into()),
            ("state".into(), if presence { "0" } else { "1" }.into()),
        ]);

        // Create broadcast action
        let action = Action {
            action: ActionType::Broadcast { extras },
            label: if presence { "Set away" } else { "Set home" }.into(),
            clear: Some(true),
        };

        // Create the notification
        let notification = Notification::new()
            .set_title("Presence")
            .set_message(if presence { "Home" } else { "Away" })
            .add_tag("house")
            .add_action(action)
            .set_priority(Priority::Low);

        if self
            .config
            .tx
            .send(Event::Ntfy(notification))
            .await
            .is_err()
        {
            warn!("There are no receivers on the event channel");
        }
    }
}

#[async_trait]
impl OnNotification for Ntfy {
    async fn on_notification(&self, notification: Notification) {
        self.send(notification).await;
    }
}
