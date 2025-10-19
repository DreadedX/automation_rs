use std::collections::HashMap;
use std::sync::Arc;

use futures::future::join_all;
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::{debug, instrument, trace};

use crate::device::Device;
use crate::event::{Event, EventChannel, OnMqtt};

pub type DeviceMap = HashMap<String, Box<dyn Device>>;

#[derive(Clone)]
pub struct DeviceManager {
    devices: Arc<RwLock<DeviceMap>>,
    event_channel: EventChannel,
}

impl DeviceManager {
    pub async fn new() -> Self {
        let (event_channel, mut event_rx) = EventChannel::new();

        let device_manager = Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            event_channel,
        };

        tokio::spawn({
            let device_manager = device_manager.clone();
            async move {
                loop {
                    if let Some(event) = event_rx.recv().await {
                        device_manager.handle_event(event).await;
                    } else {
                        todo!("Handle errors with the event channel properly")
                    }
                }
            }
        });

        device_manager
    }

    pub async fn add(&self, device: Box<dyn Device>) {
        let id = device.get_id();

        debug!(id, "Adding device");

        self.devices.write().await.insert(id, device);
    }

    pub fn event_channel(&self) -> EventChannel {
        self.event_channel.clone()
    }

    pub async fn get(&self, name: &str) -> Option<Box<dyn Device>> {
        self.devices.read().await.get(name).cloned()
    }

    pub async fn devices(&self) -> RwLockReadGuard<'_, DeviceMap> {
        self.devices.read().await
    }

    #[instrument(skip(self))]
    async fn handle_event(&self, event: Event) {
        match event {
            Event::MqttMessage(message) => {
                let devices = self.devices.read().await;
                let iter = devices.iter().map(async |(id, device)| {
                    let device: Option<&dyn OnMqtt> = device.cast();
                    if let Some(device) = device {
                        // let subscribed = device
                        //     .topics()
                        //     .iter()
                        //     .any(|topic| matches(&message.topic, topic));
                        //
                        // if subscribed {
                        trace!(id, "Handling");
                        device.on_mqtt(message.clone()).await;
                        trace!(id, "Done");
                        // }
                    }
                });

                join_all(iter).await;
            }
        }
    }
}
