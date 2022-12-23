use std::sync::{Weak, RwLock};
use log::error;

use rumqttc::{Publish, Event, Incoming, EventLoop};
use log::trace;

pub trait Listener {
    fn notify(&mut self, message: &Publish);
}

pub struct Notifier {
    listeners: Vec<Weak<RwLock<dyn Listener + Sync + Send>>>,
}

impl Notifier {
    pub fn new() -> Self {
        return Self { listeners: Vec::new() }
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

    pub async fn start(&mut self, mut eventloop: EventLoop) {
        loop {
            let notification = eventloop.poll().await;
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
    }
}
