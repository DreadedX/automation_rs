mod ikea_outlet;
mod wake_on_lan;
mod kasa_outlet;
mod audio_setup;
mod contact_sensor;

pub use self::ikea_outlet::IkeaOutlet;
pub use self::wake_on_lan::WakeOnLAN;
pub use self::kasa_outlet::KasaOutlet;
pub use self::audio_setup::AudioSetup;
pub use self::contact_sensor::ContactSensor;

use std::collections::HashMap;

use thiserror::Error;
use async_trait::async_trait;
use google_home::{GoogleHomeDevice, traits::OnOff, GoogleHome, FullfillmentError, };
use pollster::FutureExt;
use tokio::sync::{oneshot, mpsc};
use tracing::{trace, debug, span, Level};

use crate::{mqtt::{OnMqtt, self}, presence::{OnPresence, self}, light_sensor::{OnDarkness, self}};

impl_cast::impl_cast!(Device, OnMqtt);
impl_cast::impl_cast!(Device, OnPresence);
impl_cast::impl_cast!(Device, OnDarkness);
impl_cast::impl_cast!(Device, GoogleHomeDevice);
impl_cast::impl_cast!(Device, OnOff);

pub trait Device: AsGoogleHomeDevice + AsOnMqtt + AsOnPresence + AsOnDarkness + AsOnOff + std::fmt::Debug + Sync + Send {
    fn get_id(&self) -> &str;
}

// @TODO Add an inner type that we can wrap with Arc<RwLock<>> to make this type a little bit nicer
// to work with
struct Devices {
    devices: HashMap<String, DeviceBox>,
}

macro_rules! get_cast {
    ($trait:ident) => {
        paste::paste! {
            pub fn [< as_ $trait:snake s >](&mut self) -> HashMap<&str, &mut dyn $trait> {
                self.devices
                    .iter_mut()
                    .filter_map(|(id, device)| {
                        [< As $trait >]::cast_mut(device.as_mut())
                            .map(|listener| (id.as_str(), listener))
                    }).collect()
            }
        }
    };
}

#[derive(Debug)]
pub enum Command {
    Fullfillment {
        google_home: GoogleHome,
        payload: google_home::Request,
        tx: oneshot::Sender<Result<google_home::Response, FullfillmentError>>,
    },
    AddDevice {
        device: DeviceBox,
        tx: oneshot::Sender<()>
    }
}

pub type DeviceBox = Box<dyn Device>;

#[derive(Clone)]
pub struct DevicesHandle {
    tx: mpsc::Sender<Command>
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
    // @TODO Improve error type
    pub async fn fullfillment(&self, google_home: GoogleHome, payload: google_home::Request) -> Result<google_home::Response, DevicesError> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(Command::Fullfillment { google_home, payload, tx }).await?;
        Ok(rx.await??)
    }

    pub async fn add_device(&self, device: DeviceBox) -> Result<(), DevicesError> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(Command::AddDevice { device, tx }).await?;
        Ok(rx.await?)
    }
}

pub fn start(mut mqtt_rx: mqtt::Receiver, mut presence_rx: presence::Receiver, mut light_sensor_rx: light_sensor::Receiver) -> DevicesHandle {

    let mut devices = Devices { devices: HashMap::new() };

    let (tx, mut rx) = mpsc::channel(100);

    tokio::spawn(async move {
        // @TODO Handle error better
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
                // @TODO Handle receiving None better, otherwise it might constantly run doing
                // nothing
                Some(cmd) = rx.recv() => devices.handle_cmd(cmd)
            }
        }
    });

    return DevicesHandle { tx };
}

impl Devices {
    fn handle_cmd(&mut self, cmd: Command) {
        match cmd {
            Command::Fullfillment { google_home, payload, tx } => {
                let result = google_home.handle_request(payload, &mut self.as_google_home_devices());
                tx.send(result).ok();
            },
            Command::AddDevice { device, tx } => {
                self.add_device(device);

                tx.send(()).ok();
            },
        }
    }

    fn add_device(&mut self, device: DeviceBox) {
        debug!(id = device.get_id(), "Adding device");
        self.devices.insert(device.get_id().to_owned(), device);
    }

    get_cast!(OnMqtt);
    get_cast!(OnPresence);
    get_cast!(OnDarkness);
    get_cast!(GoogleHomeDevice);
}

#[async_trait]
impl OnMqtt for Devices {
    async fn on_mqtt(&mut self, message: &rumqttc::Publish) {
        self.as_on_mqtts().iter_mut().for_each(|(id, listener)| {
            let _span = span!(Level::TRACE, "on_mqtt").entered();
            trace!(id, "Handling");
            listener.on_mqtt(message).block_on();
        })
    }
}

#[async_trait]
impl OnPresence for Devices {
    async fn on_presence(&mut self, presence: bool) {
        self.as_on_presences().iter_mut().for_each(|(id, device)| {
            let _span = span!(Level::TRACE, "on_presence").entered();
            trace!(id, "Handling");
            device.on_presence(presence).block_on();
        })
    }
}

#[async_trait]
impl OnDarkness for Devices {
    async fn on_darkness(&mut self, dark: bool) {
        self.as_on_darknesss().iter_mut().for_each(|(id, device)| {
            let _span = span!(Level::TRACE, "on_darkness").entered();
            trace!(id, "Handling");
            device.on_darkness(dark).block_on();
        })
    }
}
