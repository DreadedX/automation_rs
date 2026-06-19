use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use async_trait::async_trait;
use automation_lib::action_callback::ActionCallback;
use automation_lib::device::Device;
use automation_macro::{Device, LuaDeviceConfig};
use bambulab::client::Client;
use bambulab::{Command, Message};
use google_home::errors::{self};
use google_home::traits::OnOff;
use lua_typed::Typed;
use tracing::{debug, trace};

use crate::{DebugWrap, LuaDeviceCreate};

#[derive(Debug, Clone, LuaDeviceConfig, Typed, Default)]
#[typed(as = "BambuCallbacks")]
pub struct Callbacks {
    #[device_config(from_lua, default)]
    #[typed(default)]
    pub state: ActionCallback<Bambu>,
    #[device_config(from_lua, default)]
    #[typed(default)]
    pub connected: ActionCallback<Bambu>,
}
crate::register_type!(Callbacks);

#[derive(Debug, Clone, LuaDeviceConfig, Typed)]
#[typed(as = "BambuConfig")]
pub struct Config {
    pub host: String,
    pub device_id: String,
    pub access_code: String,
    #[device_config(from_lua, default)]
    pub callbacks: Callbacks,
}
crate::register_type!(Config);

#[derive(Debug, Clone, Device)]
#[device(traits(OnOff))]
pub struct Bambu {
    config: Config,

    client: DebugWrap<Client>,

    state: Arc<AtomicBool>,
}
crate::register_device!(Bambu);

#[async_trait]
impl LuaDeviceCreate for Bambu {
    type Config = Config;
    type Error = Infallible;

    async fn create(config: Self::Config) -> Result<Self, Infallible> {
        trace!(id = config.device_id, "Setting up bambu");

        let (tx, mut rx) = tokio::sync::broadcast::channel(25);
        let client = Client::new(&config.host, &config.access_code, &config.device_id, tx);

        let state = Arc::new(AtomicBool::new(false));
        let bambu = Self {
            config,
            client: DebugWrap(client.clone()),
            state: state.clone(),
        };

        tokio::spawn({
            let mut bambu = bambu.clone();
            async move {
                // The printer might be offline so periodically try to reconnecct
                loop {
                    bambu.client.run().await.ok();

                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            }
        });

        tokio::spawn({
            let bambu = bambu.clone();
            async move {
                loop {
                    let message = rx.recv().await.unwrap();

                    match message {
                        Message::Print(data) => 'print: {
                            // Extract the state of the chamber light
                            let Some(light_report) = data.print.lights_report else {
                                break 'print;
                            };

                            let on = light_report
                                .iter()
                                .find(|report| report.node == "chamber_light")
                                .map(|report| report.mode == "on")
                                .unwrap_or(false);

                            state.store(on, std::sync::atomic::Ordering::Relaxed);

                            bambu.config.callbacks.state.call(bambu.clone()).await;
                        }
                        Message::Connected => {
                            debug!(id = bambu.config.device_id, "Connected");
                            client.publish(Command::PushAll).await.unwrap();

                            bambu.config.callbacks.connected.call(bambu.clone()).await;
                        }
                        // Ignore everything else
                        _ => {}
                    }
                }
            }
        });

        Ok(bambu)
    }
}

impl Device for Bambu {
    fn get_id(&self) -> String {
        self.config.device_id.clone()
    }
}

#[async_trait]
impl OnOff for Bambu {
    async fn on(&self) -> Result<bool, errors::ErrorCode> {
        Ok(self.state.load(std::sync::atomic::Ordering::Relaxed))
    }

    async fn set_on(&self, on: bool) -> Result<(), errors::ErrorCode> {
        // NOTE: This will error in case the printer is offline, but we don't really care in that
        // case so we just ignore the error
        self.client.publish(Command::SetChamberLight(on)).await.ok();

        Ok(())
    }
}
