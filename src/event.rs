use async_trait::async_trait;
use rumqttc::Publish;
use tokio::sync::mpsc;

use impl_cast::device_trait;

use crate::ntfy;
use crate::ntfy::Notification;

#[derive(Debug, Clone)]
pub enum Event {
    MqttMessage(Publish),
    Darkness(bool),
    Presence(bool),
    Ntfy(ntfy::Notification),
}

pub type Sender = mpsc::Sender<Event>;
pub type Receiver = mpsc::Receiver<Event>;

#[derive(Clone)]
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
#[device_trait]
pub trait OnMqtt {
    fn topics(&self) -> Vec<&str>;
    async fn on_mqtt(&mut self, message: Publish);
}

#[async_trait]
#[device_trait]
pub trait OnPresence {
    async fn on_presence(&mut self, presence: bool);
}

#[async_trait]
#[device_trait]
pub trait OnDarkness {
    async fn on_darkness(&mut self, dark: bool);
}

#[async_trait]
#[device_trait]
pub trait OnNotification {
    async fn on_notification(&mut self, notification: Notification);
}
