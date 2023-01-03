use std::sync::{Weak, RwLock};
use serde::{Serialize, Deserialize};
use tracing::{error, debug, span, Level};

use rumqttc::{Publish, Event, Incoming, EventLoop};
use tokio::task::JoinHandle;

pub trait OnMqtt {
    fn on_mqtt(&mut self, message: &Publish);
}

// @TODO Maybe rename this to make it clear it has to do with mqtt
pub struct Mqtt {
    listeners: Vec<Weak<RwLock<dyn OnMqtt + Sync + Send>>>,
    eventloop: EventLoop,
}

impl Mqtt {
    pub fn new(eventloop: EventLoop) -> Self {
        return Self { listeners: Vec::new(), eventloop }
    }

    fn notify(message: Publish, listeners: Vec<Weak<RwLock<dyn OnMqtt + Sync + Send>>>) {
        let _span = span!(Level::TRACE, "mqtt_message").entered();
        listeners.into_iter().for_each(|listener| {
            if let Some(listener) = listener.upgrade() {
                listener.write().unwrap().on_mqtt(&message);
            }
        })
    }

    pub fn add_listener<T: OnMqtt + Sync + Send + 'static>(&mut self, listener: Weak<RwLock<T>>) {
        self.listeners.push(listener);
    }

    pub fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            debug!("Listening for MQTT events");
            loop {
                let notification = self.eventloop.poll().await;
                match notification {
                    Ok(Event::Incoming(Incoming::Publish(p))) => {
                        // Remove non-existing listeners
                        self.listeners.retain(|listener| listener.strong_count() > 0);
                        // Clone the listeners
                        let listeners = self.listeners.clone();

                        // Notify might block, so we spawn a blocking task
                        tokio::task::spawn_blocking(move || {
                            Mqtt::notify(p, listeners);
                        });
                    },
                    Ok(..) => continue,
                    Err(err) => {
                        error!("{}", err);
                        break
                    },
                }
            }

            todo!("Error in MQTT (most likely lost connection to mqtt server), we need to handle these errors!");
        })
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
