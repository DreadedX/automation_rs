use async_trait::async_trait;
use rumqttc::Publish;
use tokio::sync::mpsc;

use crate::devices::Notification;

#[derive(Debug, Clone)]
pub enum Event {
    MqttMessage(Publish),
    Darkness(bool),
    Presence(bool),
    Ntfy(Notification),
}

pub type Sender = mpsc::Sender<Event>;
pub type Receiver = mpsc::Receiver<Event>;

#[derive(Clone, Debug)]
pub struct EventChannel(Sender);

impl EventChannel {
    pub fn new() -> (Self, Receiver) {
        let (tx, rx) = mpsc::channel(100);

        (Self(tx), rx)
    }

    pub fn get_tx(&self) -> Sender {
        self.0.clone()
    }
}

#[async_trait]
pub trait OnMqtt: Sync + Send {
    fn topics(&self) -> Vec<&str>;
    async fn on_mqtt(&mut self, message: Publish);
}

#[async_trait]
pub trait OnPresence: Sync + Send {
    async fn on_presence(&mut self, presence: bool);
}

#[async_trait]
pub trait OnDarkness: Sync + Send {
    async fn on_darkness(&mut self, dark: bool);
}

#[async_trait]
pub trait OnNotification: Sync + Send {
    async fn on_notification(&mut self, notification: Notification);
}
