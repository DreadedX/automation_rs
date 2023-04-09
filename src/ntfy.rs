use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{warn, error, debug};
use serde::Serialize;
use serde_repr::*;

use crate::{presence::{self, OnPresence}, config::NtfyConfig};

pub type Sender = mpsc::Sender<Notification>;
pub type Receiver = mpsc::Receiver<Notification>;

struct Ntfy {
    base_url: String,
    topic: String,
    tx: Sender,
}

#[derive(Serialize_repr)]
#[repr(u8)]
pub enum Priority {
    Min = 1,
    Low,
    Default,
    High,
    Max,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum ActionType {
    Broadcast {
        #[serde(skip_serializing_if = "HashMap::is_empty")]
        extras: HashMap<String, String>
    },
    // View,
    // Http
}

#[derive(Serialize)]
pub struct Action {
    #[serde(flatten)]
    action: ActionType,
    label: String,
    clear: Option<bool>,
}

#[derive(Serialize)]
struct NotificationFinal {
    topic: String,
    #[serde(flatten)]
    inner: Notification,
}

#[derive(Serialize)]
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
        Self { title: None, message: None, tags: Vec::new(), priority: None, actions: Vec::new() }
    }

    pub fn set_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_owned());
        self
    }

    pub fn set_message(mut self, message: &str) -> Self {
        self.message = Some(message.to_owned());
        self
    }

    pub fn add_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_owned());
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
        NotificationFinal { topic: topic.to_owned(), inner: self }
    }
}

impl Default for Notification {
    fn default() -> Self {
        Self::new()
    }
}

impl Ntfy {
    fn new(base_url: &str, topic: &str, tx: Sender) -> Self {
        Self { base_url: base_url.to_owned(), topic: topic.to_owned(), tx }
    }

    async fn send(&self, notification: Notification) {
        let notification = notification.finalize(&self.topic);
        debug!("Sending notfication");

        // Create the request
        let res = reqwest::Client::new()
            .post(self.base_url.clone())
            .json(&notification)
            .send()
            .await;

        if let Err(err) = res {
            error!("Something went wrong while sending the notifcation: {err}");
        } else if let Ok(res) = res {
            let status = res.status();
            if !status.is_success() {
                warn!("Received status {status} when sending notification");
            }
        }
    }
}

pub fn start(mut presence_rx: presence::Receiver, config: &NtfyConfig) -> Sender {
    let (tx, mut rx) = mpsc::channel(10);

    let mut ntfy = Ntfy::new(&config.url, &config.topic, tx.clone());

    tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(_) = presence_rx.changed() => {
                    let presence = *presence_rx.borrow();
                    ntfy.on_presence(presence).await;
                },
                Some(notifcation) = rx.recv() => {
                    ntfy.send(notifcation).await;
                }
            }
        }
    });

    tx
}

#[async_trait]
impl OnPresence for Ntfy {
    async fn on_presence(&mut self, presence: bool) {
        // Setup extras for the broadcast
        let extras = HashMap::from([
            ("cmd".into(), "presence".into()),
            ("state".into(), if presence { "0" } else { "1" }.into()),
        ]);

        // Create broadcast action
        let action = Action {
            action: ActionType::Broadcast { extras },
            label: if presence { "Set away" } else { "Set home" }.to_owned(),
            clear: Some(true)
        };

        // Create the notification
        let notification = Notification::new()
            .set_title("Presence")
            .set_message(if presence { "Home" } else { "Away" })
            .add_tag("house")
            .add_action(action)
            .set_priority(Priority::Low);

        self.tx.send(notification).await.ok();
    }
}
