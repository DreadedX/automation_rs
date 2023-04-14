use rumqttc::Publish;
use tokio::sync::mpsc;

use crate::ntfy;

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
