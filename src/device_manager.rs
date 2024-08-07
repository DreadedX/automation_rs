use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use futures::future::join_all;
use futures::Future;
use tokio::sync::{RwLock, RwLockReadGuard};
use tokio_cron_scheduler::{Job, JobScheduler};
use tokio_util::task::LocalPoolHandle;
use tracing::{debug, instrument, trace};

use crate::devices::Device;
use crate::event::{Event, EventChannel, OnDarkness, OnMqtt, OnNotification, OnPresence};
use crate::LUA;

pub type DeviceMap = HashMap<String, Box<dyn Device>>;

#[derive(Clone)]
pub struct DeviceManager {
    devices: Arc<RwLock<DeviceMap>>,
    event_channel: EventChannel,
    scheduler: JobScheduler,
}

impl DeviceManager {
    pub async fn new() -> Self {
        let (event_channel, mut event_rx) = EventChannel::new();

        let device_manager = Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            event_channel,
            scheduler: JobScheduler::new().await.unwrap(),
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

        device_manager.scheduler.start().await.unwrap();

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
                        let device: Option<&dyn OnMqtt> = device.cast();
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
                    let device: Option<&dyn OnDarkness> = device.cast();
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
                    let device: Option<&dyn OnPresence> = device.cast();
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
                        let device: Option<&dyn OnNotification> = device.cast();
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

fn run_schedule(
    uuid: uuid::Uuid,
    _: tokio_cron_scheduler::JobScheduler,
) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        // Lua is not Send, so we need to make sure that the task stays on the same thread
        let pool = LocalPoolHandle::new(1);
        pool.spawn_pinned(move || async move {
            let lua = LUA.lock().await;
            let f: mlua::Function = lua.named_registry_value(uuid.to_string().as_str()).unwrap();
            f.call_async::<_, ()>(()).await.unwrap();
        })
        .await
        .unwrap();
    })
}

impl mlua::UserData for DeviceManager {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_async_method("add", |_lua, this, device: Box<dyn Device>| async move {
            this.add(device).await;

            Ok(())
        });

        methods.add_async_method(
            "schedule",
            |lua, this, (schedule, f): (String, mlua::Function)| async move {
                debug!("schedule = {schedule}");
                let job = Job::new_async(schedule.as_str(), run_schedule).unwrap();

                let uuid = this.scheduler.add(job).await.unwrap();

                // Store the function in the registry
                lua.set_named_registry_value(uuid.to_string().as_str(), f)
                    .unwrap();

                Ok(())
            },
        );

        // methods.add_async_method("add_schedule", |lua, this, schedule| async {
        //     let schedule = lua.from_value(schedule)?;
        //     this.add_schedule(schedule).await;
        //     Ok(())
        // });

        methods.add_method("event_channel", |_lua, this, ()| Ok(this.event_channel()))
    }
}
