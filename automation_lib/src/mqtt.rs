use std::ops::{Deref, DerefMut};

use lua_typed::Typed;
use mlua::{FromLua, LuaSerdeExt};
use rumqttc::{AsyncClient, Event, EventLoop, Incoming};
use tracing::{debug, warn};

use crate::Module;
use crate::config::MqttConfig;
use crate::device_manager::DeviceManager;
use crate::event::{self, EventChannel};

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

    fn generate_footer() -> Option<String> {
        let mut output = String::new();

        let type_name = Self::type_name();

        output += &format!("mqtt.{type_name} = {{}}\n");
        output += &format!("---@param device_manager {}\n", DeviceManager::type_name());
        output += &format!("---@param config {}\n", MqttConfig::type_name());
        output += &format!("---@return {type_name}\n");
        output += "function mqtt.new(device_manager, config) end\n";

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

pub fn start(mut eventloop: EventLoop, event_channel: &EventChannel) {
    let tx = event_channel.get_tx();

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
}

fn create_module(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
    let mqtt = lua.create_table()?;
    let mqtt_new = lua.create_function(
        move |lua, (device_manager, config): (DeviceManager, mlua::Value)| {
            let event_channel = device_manager.event_channel();
            let config: MqttConfig = lua.from_value(config)?;

            // Create a mqtt client
            // TODO: When starting up, the devices are not yet created, this could lead to a device being out of sync
            let (client, eventloop) = AsyncClient::new(config.into(), 100);
            start(eventloop, &event_channel);

            Ok(WrappedAsyncClient(client))
        },
    )?;
    mqtt.set("new", mqtt_new)?;

    Ok(mqtt)
}

fn generate_definitions() -> String {
    let mut output = String::new();

    output += "---@meta\n\nlocal mqtt\n\n";

    output += &MqttConfig::generate_full().expect("WrappedAsyncClient should have generate_full");
    output += "\n";
    output +=
        &WrappedAsyncClient::generate_full().expect("WrappedAsyncClient should have generate_full");
    output += "\n";

    output += "return mqtt";

    output
}

inventory::submit! {Module::new("automation:mqtt", create_module, Some(generate_definitions))}
