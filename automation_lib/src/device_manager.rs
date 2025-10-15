use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use futures::Future;
use futures::future::join_all;
use lua_typed::Typed;
use tokio::sync::{RwLock, RwLockReadGuard};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, instrument, trace};

use crate::device::Device;
use crate::event::{Event, EventChannel, OnMqtt};

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

    pub async fn devices(&self) -> RwLockReadGuard<'_, DeviceMap> {
        self.devices.read().await
    }

    #[instrument(skip(self))]
    async fn handle_event(&self, event: Event) {
        match event {
            Event::MqttMessage(message) => {
                let devices = self.devices.read().await;
                let iter = devices.iter().map(async |(id, device)| {
                    let device: Option<&dyn OnMqtt> = device.cast();
                    if let Some(device) = device {
                        // let subscribed = device
                        //     .topics()
                        //     .iter()
                        //     .any(|topic| matches(&message.topic, topic));
                        //
                        // if subscribed {
                        trace!(id, "Handling");
                        device.on_mqtt(message.clone()).await;
                        trace!(id, "Done");
                        // }
                    }
                });

                join_all(iter).await;
            }
        }
    }
}

impl mlua::UserData for DeviceManager {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("add", async |_lua, this, device: Box<dyn Device>| {
            this.add(device).await;

            Ok(())
        });

        methods.add_async_method(
            "schedule",
            async |lua, this, (schedule, f): (String, mlua::Function)| {
                debug!("schedule = {schedule}");
                // This creates a function, that returns the actual job we want to run
                let create_job = {
                    let lua = lua.clone();

                    move |uuid: uuid::Uuid,
                          _: tokio_cron_scheduler::JobScheduler|
                          -> Pin<Box<dyn Future<Output = ()> + Send>> {
                        let lua = lua.clone();

                        // Create the actual function we want to run on a schedule
                        let future = async move {
                            let f: mlua::Function =
                                lua.named_registry_value(uuid.to_string().as_str()).unwrap();
                            f.call_async::<()>(()).await.unwrap();
                        };

                        Box::pin(future)
                    }
                };

                let job = Job::new_async(schedule.as_str(), create_job).unwrap();

                let uuid = this.scheduler.add(job).await.unwrap();

                // Store the function in the registry
                lua.set_named_registry_value(uuid.to_string().as_str(), f)
                    .unwrap();

                Ok(())
            },
        );

        methods.add_method("event_channel", |_lua, this, ()| Ok(this.event_channel()))
    }
}

impl Typed for DeviceManager {
    fn type_name() -> String {
        "DeviceManager".into()
    }
}
