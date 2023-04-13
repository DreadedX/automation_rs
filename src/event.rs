use rumqttc::Publish;
use tokio::sync::broadcast;

use crate::ntfy;

#[derive(Clone)]
pub enum Event {
    MqttMessage(Publish),
    Darkness(bool),
    Presence(bool),
    Ntfy(ntfy::Notification),
}

pub type Sender = broadcast::Sender<Event>;
pub type Receiver = broadcast::Receiver<Event>;

pub struct EventChannel(Sender);

impl EventChannel {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);

        Self(tx)
    }

    pub fn get_rx(&self) -> Receiver {
        self.0.subscribe()
    }

    pub fn get_tx(&self) -> Sender {
        self.0.clone()
    }
}

impl Default for EventChannel {
    fn default() -> Self {
        Self::new()
    }
}
