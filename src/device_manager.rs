use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use futures::future::join_all;
use google_home::traits::OnOff;
use mlua::{FromLua, LuaSerdeExt};
use tokio::sync::{RwLock, RwLockReadGuard};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, instrument, trace};

use crate::devices::Device;
use crate::event::{Event, EventChannel, OnDarkness, OnMqtt, OnNotification, OnPresence};
use crate::schedule::{Action, Schedule};

#[derive(Debug, FromLua, Clone)]
pub struct WrappedDevice(Arc<RwLock<Box<dyn Device>>>);

impl WrappedDevice {
    pub fn new(device: Box<dyn Device>) -> Self {
        Self(Arc::new(RwLock::new(device)))
    }
}

impl Deref for WrappedDevice {
    type Target = Arc<RwLock<Box<dyn Device>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WrappedDevice {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl mlua::UserData for WrappedDevice {}

pub type DeviceMap = HashMap<String, Arc<RwLock<Box<dyn Device>>>>;

#[derive(Debug, Clone)]
pub struct DeviceManager {
    devices: Arc<RwLock<DeviceMap>>,
    event_channel: EventChannel,
}

impl DeviceManager {
    pub fn new() -> Self {
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

    pub async fn add(&self, device: &WrappedDevice) {
        let id = device.read().await.get_id().to_owned();

        debug!(id, "Adding device");

        self.devices.write().await.insert(id, device.0.clone());
    }

    pub fn event_channel(&self) -> EventChannel {
        self.event_channel.clone()
    }

    pub async fn get(&self, name: &str) -> Option<WrappedDevice> {
        self.devices
            .read()
            .await
            .get(name)
            .cloned()
            .map(WrappedDevice)
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
                            // let subscribed = device
                            //     .topics()
                            //     .iter()
                            //     .any(|topic| matches(&message.topic, topic));
                            //
                            // if subscribed {
                            trace!(id, "Handling");
                            device.on_mqtt(message).await;
                            trace!(id, "Done");
                            // }
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
                        trace!(id, "Done");
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
                        trace!(id, "Done");
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
                            trace!(id, "Done");
                        }
                    }
                });

                join_all(iter).await;
            }
        }
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl mlua::UserData for DeviceManager {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_async_method("add", |_lua, this, device: WrappedDevice| async move {
            this.add(&device).await;

            Ok(())
        });

        methods.add_async_method("add_schedule", |lua, this, schedule| async {
            let schedule = lua.from_value(schedule)?;
            this.add_schedule(schedule).await;
            Ok(())
        });

        methods.add_method("event_channel", |_lua, this, ()| Ok(this.event_channel()))
    }
}
