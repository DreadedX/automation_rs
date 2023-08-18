use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use futures::future::join_all;
use rumqttc::{matches, AsyncClient, QoS};
use serde::Deserialize;
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::{debug, error, instrument, trace};

use crate::{
    devices::{
        As, AudioSetupConfig, ContactSensorConfig, DebugBridgeConfig, Device, HueBridgeConfig,
        HueLightConfig, IkeaOutletConfig, KasaOutletConfig, LightSensorConfig, WakeOnLANConfig,
        WasherConfig,
    },
    error::DeviceConfigError,
    event::OnDarkness,
    event::OnNotification,
    event::OnPresence,
    event::{Event, EventChannel, OnMqtt},
};

pub struct ConfigExternal<'a> {
    pub client: &'a AsyncClient,
    pub device_manager: &'a DeviceManager,
    pub event_channel: &'a EventChannel,
}

#[async_trait]
#[enum_dispatch]
pub trait DeviceConfig {
    async fn create(
        self,
        identifier: &str,
        ext: &ConfigExternal,
    ) -> Result<Box<dyn Device>, DeviceConfigError>;
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[enum_dispatch(DeviceConfig)]
pub enum DeviceConfigs {
    AudioSetup(AudioSetupConfig),
    ContactSensor(ContactSensorConfig),
    DebugBridge(DebugBridgeConfig),
    IkeaOutlet(IkeaOutletConfig),
    KasaOutlet(KasaOutletConfig),
    WakeOnLAN(WakeOnLANConfig),
    Washer(WasherConfig),
    HueBridge(HueBridgeConfig),
    HueLight(HueLightConfig),
    LightSensor(LightSensorConfig),
}

pub type WrappedDevice = Arc<RwLock<Box<dyn Device>>>;
pub type DeviceMap = HashMap<String, WrappedDevice>;

#[derive(Debug, Clone)]
pub struct DeviceManager {
    devices: Arc<RwLock<DeviceMap>>,
    client: AsyncClient,
    event_channel: EventChannel,
}

impl DeviceManager {
    pub fn new(client: AsyncClient) -> Self {
        let (event_channel, mut event_rx) = EventChannel::new();

        let device_manager = Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            client,
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
        let id = device.get_id().into();

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

    pub async fn create(
        &self,
        identifier: &str,
        device_config: DeviceConfigs,
    ) -> Result<(), DeviceConfigError> {
        let ext = ConfigExternal {
            client: &self.client,
            device_manager: self,
            event_channel: &self.event_channel,
        };

        let device = device_config.create(identifier, &ext).await?;

        self.add(device).await;

        Ok(())
    }

    pub fn event_channel(&self) -> EventChannel {
        self.event_channel.clone()
    }

    pub async fn get(&self, name: &str) -> Option<WrappedDevice> {
        self.devices.read().await.get(name).cloned()
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
