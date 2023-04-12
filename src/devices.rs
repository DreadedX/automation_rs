mod audio_setup;
mod contact_sensor;
mod ikea_outlet;
mod kasa_outlet;
mod wake_on_lan;

pub use self::audio_setup::AudioSetup;
pub use self::contact_sensor::ContactSensor;
pub use self::ikea_outlet::IkeaOutlet;
pub use self::kasa_outlet::KasaOutlet;
pub use self::wake_on_lan::WakeOnLAN;

use std::collections::HashMap;

use async_trait::async_trait;
use google_home::{traits::OnOff, FullfillmentError, GoogleHome, GoogleHomeDevice};
use pollster::FutureExt;
use rumqttc::{matches, AsyncClient, QoS};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, trace};

use crate::{
    light_sensor::{self, OnDarkness},
    mqtt::{self, OnMqtt},
    presence::{self, OnPresence},
};

#[impl_cast::device(As: OnMqtt + OnPresence + OnDarkness + GoogleHomeDevice + OnOff)]
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

pub fn start(
    mut mqtt_rx: mqtt::Receiver,
    mut presence_rx: presence::Receiver,
    mut light_sensor_rx: light_sensor::Receiver,
    client: AsyncClient,
) -> DevicesHandle {
    let mut devices = Devices {
        devices: HashMap::new(),
        client,
    };

    let (tx, mut rx) = mpsc::channel(100);

    tokio::spawn(async move {
        // TODO: Handle error better
        loop {
            tokio::select! {
                Ok(message) = mqtt_rx.recv() => {
                    devices.on_mqtt(&message).await;
                }
                Ok(_) = presence_rx.changed() => {
                    let presence = *presence_rx.borrow();
                    devices.on_presence(presence).await;
                }
                Ok(_) = light_sensor_rx.changed() => {
                    let darkness = *light_sensor_rx.borrow();
                    devices.on_darkness(darkness).await;
                }
                // TODO: Handle receiving None better, otherwise it might constantly run doing
                // nothing
                Some(cmd) = rx.recv() => devices.handle_cmd(cmd).await
            }
        }
    });

    DevicesHandle { tx }
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

#[async_trait]
impl OnMqtt for Devices {
    fn topics(&self) -> Vec<&str> {
        Vec::new()
    }

    #[tracing::instrument(skip_all)]
    async fn on_mqtt(&mut self, message: &rumqttc::Publish) {
        self.get::<dyn OnMqtt>()
            .iter_mut()
            .for_each(|(id, listener)| {
                let subscribed = listener
                    .topics()
                    .iter()
                    .any(|topic| matches(&message.topic, topic));

                if subscribed {
                    trace!(id, "Handling");
                    listener.on_mqtt(message).block_on();
                }
            })
    }
}

#[async_trait]
impl OnPresence for Devices {
    #[tracing::instrument(skip(self))]
    async fn on_presence(&mut self, presence: bool) {
        self.get::<dyn OnPresence>()
            .iter_mut()
            .for_each(|(id, device)| {
                trace!(id, "Handling");
                device.on_presence(presence).block_on();
            })
    }
}

#[async_trait]
impl OnDarkness for Devices {
    #[tracing::instrument(skip(self))]
    async fn on_darkness(&mut self, dark: bool) {
        self.get::<dyn OnDarkness>()
            .iter_mut()
            .for_each(|(id, device)| {
                trace!(id, "Handling");
                device.on_darkness(dark).block_on();
            })
    }
}
