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

use async_trait::async_trait;
use google_home::{GoogleHomeDevice, traits::OnOff, GoogleHome};
use pollster::FutureExt;
use tokio::sync::{oneshot, mpsc};
use tracing::{trace, debug, span, Level};

use crate::{mqtt::{OnMqtt, self}, presence::{OnPresence, self}, light_sensor::{OnDarkness, self}};

impl_cast::impl_cast!(Device, OnMqtt);
impl_cast::impl_cast!(Device, OnPresence);
impl_cast::impl_cast!(Device, OnDarkness);
impl_cast::impl_cast!(Device, GoogleHomeDevice);
impl_cast::impl_cast!(Device, OnOff);

pub trait Device: AsGoogleHomeDevice + AsOnMqtt + AsOnPresence + AsOnDarkness + AsOnOff + std::fmt::Debug {
    fn get_id(&self) -> String;
}

// @TODO Add an inner type that we can wrap with Arc<RwLock<>> to make this type a little bit nicer
// to work with
struct Devices {
    devices: HashMap<String, DeviceBox>,
}

macro_rules! get_cast {
    ($trait:ident) => {
        paste::paste! {
            pub fn [< as_ $trait:snake s >](&mut self) -> HashMap<String, &mut dyn $trait> {
                self.devices
                    .iter_mut()
                    .filter_map(|(id, device)| {
                        if let Some(listener) = [< As $trait >]::cast_mut(device.as_mut()) {
                            return Some((id.clone(), listener));
                        };
                        return None;
                    }).collect()
            }
        }
    };
}

#[derive(Debug)]
enum Command {
    Fullfillment {
        google_home: GoogleHome,
        payload: google_home::Request,
        tx: oneshot::Sender<google_home::Response>,
    },
    AddDevice {
        device: DeviceBox,
        tx: oneshot::Sender<()>
    }
}

pub type DeviceBox = Box<dyn Device + Sync + Send>;

#[derive(Clone)]
pub struct DeviceHandle {
    tx: mpsc::Sender<Command>
}

impl DeviceHandle {
    // @TODO Improve error type
    pub async fn fullfillment(&self, google_home: GoogleHome, payload: google_home::Request) -> Result<google_home::Response, oneshot::error::RecvError> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(Command::Fullfillment { google_home, payload, tx }).await.unwrap();
        rx.await
    }

    pub async fn add_device(&self, device: DeviceBox) {
        let (tx, rx) = oneshot::channel();
        self.tx.send(Command::AddDevice { device, tx }).await.unwrap();
        rx.await.ok();
    }
}

pub fn start(mut mqtt_rx: mqtt::Receiver, mut presence_rx: presence::Receiver, mut light_sensor_rx: light_sensor::Receiver) -> DeviceHandle {

    let mut devices = Devices { devices: HashMap::new() };

    let (tx, mut rx) = mpsc::channel(100);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                res = mqtt_rx.changed() => {
                    if !res.is_ok() {
                        break;
                    }

                    // @TODO Not ideal that we have to clone here, but not sure how to work around that
                    let message = mqtt_rx.borrow().clone();
                    if let Some(message) = message {
                        devices.on_mqtt(&message).await;
                    }
                }
                res = presence_rx.changed() => {
                    if !res.is_ok() {
                        break;
                    }

                    let presence = *presence_rx.borrow();
                    devices.on_presence(presence).await;
                }
                res = light_sensor_rx.changed() => {
                    if !res.is_ok() {
                        break;
                    }

                    let darkness = *light_sensor_rx.borrow();
                    devices.on_darkness(darkness).await;
                }
                Some(cmd) = rx.recv() => devices.handle_cmd(cmd)
            }
        }

        unreachable!("Did not expect this");
    });

    return DeviceHandle { tx };
}

impl Devices {
    fn handle_cmd(&mut self, cmd: Command) {
        match cmd {
            Command::Fullfillment { google_home, payload, tx } => {
                let result = google_home.handle_request(payload, &mut self.as_google_home_devices()).unwrap();
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
        self.devices.insert(device.get_id(), device);
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
