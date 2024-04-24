use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use futures::future::join_all;
use google_home::traits::OnOff;
use rumqttc::{matches, AsyncClient, QoS};
use tokio::sync::{RwLock, RwLockReadGuard};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, error, instrument, trace};

use crate::devices::Device;
use crate::error::DeviceConfigError;
use crate::event::{Event, EventChannel, OnDarkness, OnMqtt, OnNotification, OnPresence};
use crate::schedule::{Action, Schedule};

pub struct ConfigExternal<'a> {
    pub client: &'a AsyncClient,
    pub device_manager: &'a DeviceManager,
    pub event_channel: &'a EventChannel,
}

#[async_trait]
#[enum_dispatch]
pub trait DeviceConfig {
    async fn create(
        &self,
        identifier: &str,
        ext: &ConfigExternal,
    ) -> Result<Box<dyn Device>, DeviceConfigError>;
}
impl mlua::UserData for Box<dyn DeviceConfig> {}

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

    // TODO: This function is currently extremely cursed...
    pub async fn add_schedule(&self, schedule: Schedule) {
        let sched = JobScheduler::new().await.unwrap();

        for (when, actions) in schedule {
            let manager = self.clone();
            sched
                .add(
                    Job::new_async(when.as_str(), move |_uuid, _l| {
                        let actions = actions.clone();
                        let manager = manager.clone();

                        Box::pin(async move {
                            for (action, targets) in actions {
                                for target in targets {
                                    let device = manager.get(&target).await.unwrap();
                                    match action {
                                        Action::On => {
                                            let mut device = device.write().await;
                                            let device: Option<&mut dyn OnOff> =
                                                device.as_mut().cast_mut();

                                            if let Some(device) = device {
                                                device.set_on(true).await.unwrap();
                                            }
                                        }
                                        Action::Off => {
                                            let mut device = device.write().await;
                                            let device: Option<&mut dyn OnOff> =
                                                device.as_mut().cast_mut();

                                            if let Some(device) = device {
                                                device.set_on(false).await.unwrap();
                                            }
                                        }
                                    }
                                }
                            }
                        })
                    })
                    .unwrap(),
                )
                .await
                .unwrap();
        }

        sched.start().await.unwrap();
    }

    pub async fn add(&self, device: Box<dyn Device>) {
        let id = device.get_id().into();

        debug!(id, "Adding device");

        // If the device listens to mqtt, subscribe to the topics
        if let Some(device) = device.as_ref().cast() as Option<&dyn OnMqtt> {
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
                        let device: Option<&mut dyn OnMqtt> = device.as_mut().cast_mut();
                        if let Some(device) = device {
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
                    let device: Option<&mut dyn OnDarkness> = device.as_mut().cast_mut();
                    if let Some(device) = device {
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
                    let device: Option<&mut dyn OnPresence> = device.as_mut().cast_mut();
                    if let Some(device) = device {
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
                        let device: Option<&mut dyn OnNotification> = device.as_mut().cast_mut();
                        if let Some(device) = device {
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

impl mlua::UserData for DeviceManager {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_async_method(
            "create",
            |_lua, this, (identifier, config): (String, mlua::Value)| async move {
                // TODO: Handle the error here properly
                let config: Box<dyn DeviceConfig> = config.as_userdata().unwrap().take()?;

                let ext = ConfigExternal {
                    client: &this.client,
                    device_manager: this,
                    event_channel: &this.event_channel,
                };

                let device = config
                    .create(&identifier, &ext)
                    .await
                    .map_err(mlua::ExternalError::into_lua_err)?;

                let id = device.get_id().to_owned();

                this.add(device).await;

                Ok(id)
            },
        )
    }
}
