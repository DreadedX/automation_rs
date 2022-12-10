use std::{rc::Weak, cell::RefCell};

use rumqttc::{Publish, Connection, Event, Incoming};

pub trait Listener {
    fn notify(&mut self, message: &Publish);
}

pub struct Notifier {
    listeners: Vec<Weak<RefCell<dyn Listener>>>,
}

impl Notifier {
    pub fn new() -> Self {
        return Self { listeners: Vec::new() }
    }

    fn notify(&mut self, message: Publish) {
        self.listeners.retain(|listener| {
            if let Some(listener) = listener.upgrade() {
                listener.borrow_mut().notify(&message);
                return true;
            }

            return false;
        })
    }

    pub fn add_listener<T: Listener + 'static>(&mut self, listener: Weak<RefCell<T>>) {
        self.listeners.push(listener);
    }

    pub fn start(&mut self, mut connection: Connection) {
        for notification in connection.iter() {
            match notification {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    println!("{:?}", p);
                    self.notify(p);
                },
                Ok(..) => continue,
                Err(err) => {
                    eprintln!("{}", err);
                    break
                },
            }
        }
    }
}
