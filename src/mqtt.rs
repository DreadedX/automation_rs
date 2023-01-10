use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use tracing::{error, debug};

use rumqttc::{Publish, Event, Incoming, EventLoop};
use tokio::sync::watch;

#[async_trait]
pub trait OnMqtt {
    async fn on_mqtt(&mut self, message: &Publish);
}

pub type Receiver = watch::Receiver<Option<Publish>>;
type Sender = watch::Sender<Option<Publish>>;

pub struct Mqtt {
    tx: Sender,
    eventloop: EventLoop,
}

impl Mqtt {
    pub fn new(eventloop: EventLoop) -> Self {
        let (tx, _rx) = watch::channel(None);
        Self { tx, eventloop }
    }

    pub fn subscribe(&self) -> Receiver {
        self.tx.subscribe()
    }

    pub fn start(mut self) {
        tokio::spawn(async move {
            debug!("Listening for MQTT events");
            loop {
                let notification = self.eventloop.poll().await;
                match notification {
                    Ok(Event::Incoming(Incoming::Publish(p))) => {
                        self.tx.send(Some(p)).ok();
                    },
                    Ok(..) => continue,
                    Err(err) => {
                        error!("{}", err);
                        break
                    },
                }
            }

            todo!("Error in MQTT (most likely lost connection to mqtt server), we need to handle these errors!");
        });
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OnOffMessage {
    state: String
}

impl OnOffMessage {
    pub fn new(state: bool) -> Self {
        Self { state: if state {"ON"} else {"OFF"}.into() }
    }

    pub fn state(&self) -> bool {
        self.state == "ON"
    }
}

impl TryFrom<&Publish> for OnOffMessage {
    type Error = anyhow::Error;

    fn try_from(message: &Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(anyhow::anyhow!("Invalid message payload received: {:?}", message.payload)))
    }
}

#[derive(Debug, Deserialize)]
pub struct ActivateMessage {
    activate: bool
}

impl ActivateMessage {
    pub fn activate(&self) -> bool {
        self.activate
    }
}

impl TryFrom<&Publish> for ActivateMessage {
    type Error = anyhow::Error;

    fn try_from(message: &Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(anyhow::anyhow!("Invalid message payload received: {:?}", message.payload)))
    }
}

#[derive(Debug, Deserialize, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RemoteAction {
    On,
    Off,
    BrightnessMoveUp,
    BrightnessMoveDown,
    BrightnessStop,
}

#[derive(Debug, Deserialize)]
pub struct RemoteMessage {
    action: RemoteAction
}

impl RemoteMessage {
    pub fn action(&self) -> RemoteAction {
        self.action
    }
}

impl TryFrom<&Publish> for RemoteMessage {
    type Error = anyhow::Error;

    fn try_from(message: &Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(anyhow::anyhow!("Invalid message payload received: {:?}", message.payload)))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PresenceMessage {
    state: bool
}

impl PresenceMessage {
    pub fn new(state: bool) -> Self {
        Self { state }
    }

    pub fn present(&self) -> bool {
        self.state
    }
}

impl TryFrom<&Publish> for PresenceMessage {
    type Error = anyhow::Error;

    fn try_from(message: &Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(anyhow::anyhow!("Invalid message payload received: {:?}", message.payload)))
    }
}

#[derive(Debug, Deserialize)]
pub struct BrightnessMessage {
    illuminance: isize,
}

impl BrightnessMessage {
    pub fn illuminance(&self) -> isize {
        self.illuminance
    }
}

impl TryFrom<&Publish> for BrightnessMessage {
    type Error = anyhow::Error;

    fn try_from(message: &Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(anyhow::anyhow!("Invalid message payload received: {:?}", message.payload)))
    }
}

#[derive(Debug, Deserialize)]
pub struct ContactMessage {
    contact: bool,
}

impl ContactMessage {
    pub fn is_closed(&self) -> bool {
        self.contact
    }
}

impl TryFrom<&Publish> for ContactMessage {
    type Error = anyhow::Error;

    fn try_from(message: &Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(anyhow::anyhow!("Invalid message payload received: {:?}", message.payload)))
    }
}

