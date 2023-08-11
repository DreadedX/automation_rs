use std::collections::HashMap;
use std::sync::Arc;

use futures::future::join_all;
use rumqttc::{matches, AsyncClient, QoS};
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::{debug, error, instrument, trace};

use crate::{
    devices::{As, Device},
    event::OnDarkness,
    event::OnNotification,
    event::OnPresence,
    event::{Event, EventChannel, OnMqtt},
};

pub type DeviceMap = HashMap<String, Arc<RwLock<Box<dyn Device>>>>;

#[derive(Debug, Clone)]
pub struct DeviceManager {
    devices: Arc<RwLock<DeviceMap>>,
    client: AsyncClient,
}

impl DeviceManager {
    pub fn new(client: AsyncClient) -> Self {
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            client,
        }
    }

    pub fn start(&self) -> EventChannel {
        let (event_channel, mut event_rx) = EventChannel::new();

        let devices = self.clone();
        tokio::spawn(async move {
            loop {
                if let Some(event) = event_rx.recv().await {
                    devices.handle_event(event).await;
                } else {
                    todo!("Handle errors with the event channel properly")
                }
            }
        });

        event_channel
    }

    pub async fn add(&self, device: Box<dyn Device>) {
        let id = device.get_id().to_owned();

        debug!(id, "Adding device");

        // If the device listens to mqtt, subscribe to the topics
        if let Some(device) = As::<dyn OnMqtt>::cast(device.as_ref()) {
            for topic in device.topics() {
                trace!(id, topic, "Subscribing to topic");
                if let Err(err) = self.client.subscribe(topic, QoS::AtLeastOnce).await {
                    // NOTE: Pretty sure that this can only happen if the mqtt client if no longer
                    // running
                    error!(id, topic, "Failed to subscribe to topic: {err}");
                }
            }
        }

        // Wrap the device
        let device = Arc::new(RwLock::new(device));

        self.devices.write().await.insert(id, device);
    }

    pub async fn devices(&self) -> RwLockReadGuard<DeviceMap> {
        self.devices.read().await
    }

    #[instrument(skip(self))]
    async fn handle_event(&self, event: Event) {
        match event {
            Event::MqttMessage(message) => {
                let devices = self.devices.read().await;
                let iter = devices.iter().map(|(id, device)| {
                    let message = message.clone();
                    async move {
                        let mut device = device.write().await;
                        let device = device.as_mut();
                        if let Some(device) = As::<dyn OnMqtt>::cast_mut(device) {
                            let subscribed = device
                                .topics()
                                .iter()
                                .any(|topic| matches(&message.topic, topic));

                            if subscribed {
                                trace!(id, "Handling");
                                device.on_mqtt(message).await;
                            }
                        }
                    }
                });

                join_all(iter).await;
            }
            Event::Darkness(dark) => {
                let devices = self.devices.read().await;
                let iter = devices.iter().map(|(id, device)| async move {
                    let mut device = device.write().await;
                    let device = device.as_mut();
                    if let Some(device) = As::<dyn OnDarkness>::cast_mut(device) {
                        trace!(id, "Handling");
                        device.on_darkness(dark).await;
                    }
                });

                join_all(iter).await;
            }
            Event::Presence(presence) => {
                let devices = self.devices.read().await;
                let iter = devices.iter().map(|(id, device)| async move {
                    let mut device = device.write().await;
                    let device = device.as_mut();
                    if let Some(device) = As::<dyn OnPresence>::cast_mut(device) {
                        trace!(id, "Handling");
                        device.on_presence(presence).await;
                    }
                });

                join_all(iter).await;
            }
            Event::Ntfy(notification) => {
                let devices = self.devices.read().await;
                let iter = devices.iter().map(|(id, device)| {
                    let notification = notification.clone();
                    async move {
                        let mut device = device.write().await;
                        let device = device.as_mut();
                        if let Some(device) = As::<dyn OnNotification>::cast_mut(device) {
                            trace!(id, "Handling");
                            device.on_notification(notification).await;
                        }
                    }
                });

                join_all(iter).await;
            }
        }
    }
}
