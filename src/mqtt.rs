use std::ops::{Deref, DerefMut};

use mlua::FromLua;
use rumqttc::{AsyncClient, Event, EventLoop, Incoming};
use tracing::{debug, warn};

use crate::event::{self, EventChannel};

#[derive(Debug, Clone, FromLua)]
pub struct WrappedAsyncClient(pub AsyncClient);

impl Deref for WrappedAsyncClient {
    type Target = AsyncClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WrappedAsyncClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl mlua::UserData for WrappedAsyncClient {}

pub fn start(mut eventloop: EventLoop, event_channel: &EventChannel) {
    let tx = event_channel.get_tx();

    tokio::spawn(async move {
        debug!("Listening for MQTT events");
        loop {
            let notification = eventloop.poll().await;
            match notification {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    tx.send(event::Event::MqttMessage(p)).await.ok();
                }
                Ok(..) => continue,
                Err(err) => {
                    // Something has gone wrong
                    // We stay in the loop as that will attempt to reconnect
                    warn!("{}", err);
                }
            }
        }
    });
}
