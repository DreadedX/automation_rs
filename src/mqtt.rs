use std::sync::{Weak, RwLock};
use tracing::{error, debug, trace, span, Level};

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

    fn notify(&mut self, message: Publish) {
        self.listeners.retain(|listener| {
            if let Some(listener) = listener.upgrade() {
                listener.write().unwrap().on_mqtt(&message);
                return true;
            } else {
                trace!("Removing listener...");
            }

            return false;
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
                        // Could cause problems in async
                        let _span = span!(Level::TRACE, "mqtt_message").entered();
                        self.notify(p);
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
