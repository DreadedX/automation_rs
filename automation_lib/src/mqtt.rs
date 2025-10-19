use std::ops::{Deref, DerefMut};
use std::time::Duration;

use automation_macro::LuaDeviceConfig;
use lua_typed::Typed;
use mlua::FromLua;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, Transport};
use serde::Deserialize;
use tracing::{debug, warn};

use crate::event::{self, EventChannel};

#[derive(Debug, Clone, LuaDeviceConfig, Deserialize, Typed)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub client_name: String,
    pub username: String,
    pub password: String,
    #[serde(default)]
    #[typed(default)]
    pub tls: bool,
}

impl From<MqttConfig> for MqttOptions {
    fn from(value: MqttConfig) -> Self {
        let mut mqtt_options = MqttOptions::new(value.client_name, value.host, value.port);
        mqtt_options.set_credentials(value.username, value.password);
        mqtt_options.set_keep_alive(Duration::from_secs(5));

        if value.tls {
            mqtt_options.set_transport(Transport::tls_with_default_config());
        }

        mqtt_options
    }
}

#[derive(Debug, Clone, FromLua)]
pub struct WrappedAsyncClient(pub AsyncClient);

impl Typed for WrappedAsyncClient {
    fn type_name() -> String {
        "AsyncClient".into()
    }

    fn generate_header() -> Option<String> {
        let type_name = Self::type_name();
        Some(format!("---@class {type_name}\nlocal {type_name}\n"))
    }

    fn generate_members() -> Option<String> {
        let mut output = String::new();

        let type_name = Self::type_name();

        output += &format!(
            "---@async\n---@param topic string\n---@param message table?\nfunction {type_name}:send_message(topic, message) end\n"
        );

        Some(output)
    }
}

impl Deref for WrappedAsyncClient {
    type Target = AsyncClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WrappedAsyncClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl mlua::UserData for WrappedAsyncClient {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method(
            "send_message",
            async |_lua, this, (topic, message): (String, mlua::Value)| {
                // serde_json converts nil => "null", but we actually want nil to send an empty
                // message
                let message = if message.is_nil() {
                    "".into()
                } else {
                    serde_json::to_string(&message).unwrap()
                };

                debug!("message = {message}");

                this.0
                    .publish(topic, rumqttc::QoS::AtLeastOnce, true, message)
                    .await
                    .unwrap();

                Ok(())
            },
        );
    }
}

pub fn start(config: MqttConfig, event_channel: &EventChannel) -> WrappedAsyncClient {
    let tx = event_channel.get_tx();
    let (client, mut eventloop) = AsyncClient::new(config.into(), 100);

    tokio::spawn(async move {
        debug!("Listening for MQTT events");
        loop {
            let notification = eventloop.poll().await;
            match notification {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    tx.send(event::Event::MqttMessage(p)).await.ok();
                }
                Ok(..) => continue,
                Err(err) => {
                    // Something has gone wrong
                    // We stay in the loop as that will attempt to reconnect
                    warn!("{}", err);
                }
            }
        }
    });

    WrappedAsyncClient(client)
}
