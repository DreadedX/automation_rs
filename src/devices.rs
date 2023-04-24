mod audio_setup;
mod contact_sensor;
mod debug_bridge;
mod hue_bridge;
mod ikea_outlet;
mod kasa_outlet;
mod light_sensor;
mod presence;
mod wake_on_lan;

pub use self::audio_setup::AudioSetup;
pub use self::contact_sensor::ContactSensor;
pub use self::debug_bridge::{DebugBridge, DebugBridgeConfig};
pub use self::hue_bridge::{HueBridge, HueBridgeConfig};
pub use self::ikea_outlet::IkeaOutlet;
pub use self::kasa_outlet::KasaOutlet;
pub use self::light_sensor::{LightSensor, LightSensorConfig};
pub use self::presence::{Presence, PresenceConfig, DEFAULT_PRESENCE};
pub use self::wake_on_lan::WakeOnLAN;

use std::collections::HashMap;

use futures::future::join_all;
use google_home::{traits::OnOff, FullfillmentError, GoogleHome, GoogleHomeDevice};
use rumqttc::{matches, AsyncClient, QoS};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, instrument, trace};

use crate::{
    event::OnDarkness,
    event::OnMqtt,
    event::OnNotification,
    event::OnPresence,
    event::{Event, EventChannel},
};

#[impl_cast::device(As: OnMqtt + OnPresence + OnDarkness + OnNotification + GoogleHomeDevice + OnOff)]
pub trait Device: std::fmt::Debug + Sync + Send {
    fn get_id(&self) -> &str;
}

// TODO: Add an inner type that we can wrap with Arc<RwLock<>> to make this type a little bit nicer
// to work with
#[derive(Debug)]
struct Devices {
    devices: HashMap<String, Box<dyn Device>>,
    client: AsyncClient,
}

#[derive(Debug)]
pub enum Command {
    Fullfillment {
        google_home: GoogleHome,
        payload: google_home::Request,
        tx: oneshot::Sender<Result<google_home::Response, FullfillmentError>>,
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
    pub async fn fullfillment(
        &self,
        google_home: GoogleHome,
        payload: google_home::Request,
    ) -> Result<google_home::Response, DevicesError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Fullfillment {
                google_home,
                payload,
                tx,
            })
            .await?;
        Ok(rx.await??)
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
            Command::Fullfillment {
                google_home,
                payload,
                tx,
            } => {
                let result =
                    google_home.handle_request(payload, &mut self.get::<dyn GoogleHomeDevice>());
                tx.send(result).ok();
            }
            Command::AddDevice { device, tx } => {
                self.add_device(device).await;

                tx.send(()).ok();
            }
        }
    }

    async fn add_device(&mut self, device: Box<dyn Device>) {
        let id = device.get_id();
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

        self.devices.insert(device.get_id().to_owned(), device);
    }

    #[instrument(skip(self))]
    async fn handle_event(&mut self, event: Event) {
        match event {
            Event::MqttMessage(message) => {
                let iter = self.get::<dyn OnMqtt>().into_iter().map(|(id, listener)| {
                    let message = message.clone();
                    async move {
                        let subscribed = listener
                            .topics()
                            .iter()
                            .any(|topic| matches(&message.topic, topic));

                        if subscribed {
                            trace!(id, "Handling");
                            listener.on_mqtt(message).await;
                        }
                    }
                });

                join_all(iter).await;
            }
            Event::Darkness(dark) => {
                let iter =
                    self.get::<dyn OnDarkness>()
                        .into_iter()
                        .map(|(id, device)| async move {
                            trace!(id, "Handling");
                            device.on_darkness(dark).await;
                        });

                join_all(iter).await;
            }
            Event::Presence(presence) => {
                let iter =
                    self.get::<dyn OnPresence>()
                        .into_iter()
                        .map(|(id, device)| async move {
                            trace!(id, "Handling");
                            device.on_presence(presence).await;
                        });

                join_all(iter).await;
            }
            Event::Ntfy(notification) => {
                let iter = self
                    .get::<dyn OnNotification>()
                    .into_iter()
                    .map(|(id, device)| {
                        let notification = notification.clone();
                        async move {
                            trace!(id, "Handling");
                            device.on_notification(notification).await;
                        }
                    });

                join_all(iter).await;
            }
        }
    }

    fn get<T>(&mut self) -> HashMap<&str, &mut T>
    where
        T: ?Sized + 'static,
        (dyn Device): As<T>,
    {
        self.devices
            .iter_mut()
            .filter_map(|(id, device)| As::<T>::cast_mut(device.as_mut()).map(|t| (id.as_str(), t)))
            .collect()
    }
}
