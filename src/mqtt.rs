use std::sync::{Weak, RwLock};
use log::{error, debug};

use rumqttc::{Publish, Event, Incoming, EventLoop};
use log::trace;
use tokio::task::JoinHandle;

pub trait Listener {
    fn notify(&mut self, message: &Publish);
}

// @TODO Maybe rename this to make it clear it has to do with mqtt
pub struct Notifier {
    listeners: Vec<Weak<RwLock<dyn Listener + Sync + Send>>>,
    eventloop: EventLoop,
}

impl Notifier {
    pub fn new(eventloop: EventLoop) -> Self {
        return Self { listeners: Vec::new(), eventloop }
    }

    fn notify(&mut self, message: Publish) {
        self.listeners.retain(|listener| {
            if let Some(listener) = listener.upgrade() {
                listener.write().unwrap().notify(&message);
                return true;
            }

            return false;
        })
    }

    pub fn add_listener<T: Listener + Sync + Send + 'static>(&mut self, listener: Weak<RwLock<T>>) {
        self.listeners.push(listener);
    }

    pub fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            debug!("Listening for MQTT events");
            loop {
                let notification = self.eventloop.poll().await;
                match notification {
                    Ok(Event::Incoming(Incoming::Publish(p))) => {
                        trace!("{:?}", p);
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
