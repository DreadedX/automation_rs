mod audio_setup;
mod contact_sensor;
mod debug_bridge;
mod hue_bridge;
mod ikea_outlet;
mod kasa_outlet;
mod light_sensor;
mod ntfy;
mod presence;
mod wake_on_lan;

pub use self::audio_setup::AudioSetup;
pub use self::contact_sensor::ContactSensor;
pub use self::debug_bridge::{DebugBridge, DebugBridgeConfig};
pub use self::hue_bridge::{HueBridge, HueBridgeConfig};
pub use self::ikea_outlet::IkeaOutlet;
pub use self::kasa_outlet::KasaOutlet;
pub use self::light_sensor::{LightSensor, LightSensorConfig};
pub use self::ntfy::{Notification, Ntfy};
pub use self::presence::{Presence, PresenceConfig, DEFAULT_PRESENCE};
pub use self::wake_on_lan::WakeOnLAN;

use std::collections::HashMap;
use std::sync::Arc;

use futures::future::join_all;
use google_home::device::AsGoogleHomeDevice;
use google_home::{traits::OnOff, FullfillmentError};
use rumqttc::{matches, AsyncClient, QoS};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{debug, error, instrument, trace};

use crate::{
    event::OnDarkness,
    event::OnMqtt,
    event::OnNotification,
    event::OnPresence,
    event::{Event, EventChannel},
};

#[impl_cast::device(As: OnMqtt + OnPresence + OnDarkness + OnNotification + OnOff)]
pub trait Device: AsGoogleHomeDevice + std::fmt::Debug + Sync + Send {
    fn get_id(&self) -> &str;
}

pub type DeviceMap = HashMap<String, Arc<RwLock<Box<dyn Device>>>>;

// TODO: Add an inner type that we can wrap with Arc<RwLock<>> to make this type a little bit nicer
// to work with
#[derive(Debug)]
struct Devices {
    devices: DeviceMap,
    client: AsyncClient,
}

#[derive(Debug)]
pub enum Command {
    Fullfillment {
        tx: oneshot::Sender<DeviceMap>,
    },
    AddDevice {
        device: Box<dyn Device>,
        tx: oneshot::Sender<()>,
    },
}

#[derive(Clone)]
pub struct DevicesHandle {
    tx: mpsc::Sender<Command>,
}

#[derive(Debug, Error)]
pub enum DevicesError {
    #[error(transparent)]
    FullfillmentError(#[from] FullfillmentError),
    #[error(transparent)]
    SendError(#[from] tokio::sync::mpsc::error::SendError<Command>),
    #[error(transparent)]
    RecvError(#[from] tokio::sync::oneshot::error::RecvError),
}

impl DevicesHandle {
    // TODO: Improve error type
    pub async fn fullfillment(&self) -> Result<DeviceMap, DevicesError> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(Command::Fullfillment { tx }).await?;
        Ok(rx.await?)
    }

    pub async fn add_device(&self, device: Box<dyn Device>) -> Result<(), DevicesError> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(Command::AddDevice { device, tx }).await?;
        Ok(rx.await?)
    }
}

pub fn start(client: AsyncClient) -> (DevicesHandle, EventChannel) {
    let mut devices = Devices {
        devices: HashMap::new(),
        client,
    };

    let (event_channel, mut event_rx) = EventChannel::new();
    let (tx, mut rx) = mpsc::channel(100);

    tokio::spawn(async move {
        // TODO: Handle error better
        loop {
            tokio::select! {
                event = event_rx.recv() => {
                    if event.is_none() {
                        todo!("Handle errors with the event channel properly")
                    }
                    devices.handle_event(event.unwrap()).await;
                }
                // TODO: Handle receiving None better, otherwise it might constantly run doing
                // nothing
                cmd = rx.recv() => {
                    if cmd.is_none() {
                        todo!("Handle errors with the cmd channel properly")
                    }
                    devices.handle_cmd(cmd.unwrap()).await;
                }
            }
        }
    });

    (DevicesHandle { tx }, event_channel)
}

impl Devices {
    async fn handle_cmd(&mut self, cmd: Command) {
        match cmd {
            Command::Fullfillment { tx } => {
                tx.send(self.devices.clone()).ok();
            }
            Command::AddDevice { device, tx } => {
                self.add_device(device).await;

                tx.send(()).ok();
            }
        }
    }

    async fn add_device(&mut self, device: Box<dyn Device>) {
        let id = device.get_id().to_owned();

        let device = Arc::new(RwLock::new(device));
        {
            let device = device.read().await;

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
        }

        self.devices.insert(id, device);
    }

    #[instrument(skip(self))]
    async fn handle_event(&mut self, event: Event) {
        match event {
            Event::MqttMessage(message) => {
                let iter = self.devices.iter().map(|(id, device)| {
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
                let iter = self.devices.iter().map(|(id, device)| async move {
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
                let iter = self.devices.iter().map(|(id, device)| async move {
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
                let iter = self.devices.iter().map(|(id, device)| {
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
