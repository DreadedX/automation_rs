#![feature(iter_intersperse)]
mod config;
mod secret;
mod web;

use std::net::SocketAddr;
use std::path::Path;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use ::config::{Environment, File};
use automation_lib::config::{FulfillmentConfig, MqttConfig};
use automation_lib::device_manager::DeviceManager;
use automation_lib::helpers;
use automation_lib::mqtt::{self, WrappedAsyncClient};
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use config::Config;
use dotenvy::dotenv;
use google_home::{GoogleHome, Request, Response};
use mlua::LuaSerdeExt;
use rumqttc::AsyncClient;
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};
use web::{ApiError, User};

use crate::secret::EnvironmentSecretFile;

#[derive(Clone)]
struct AppState {
    pub openid_url: String,
    pub device_manager: DeviceManager,
}

impl FromRef<AppState> for String {
    fn from_ref(input: &AppState) -> Self {
        input.openid_url.clone()
    }
}

#[tokio::main]
async fn main() {
    if let Err(err) = app().await {
        error!("Error: {err}");
        let mut cause = err.source();
        while let Some(c) = cause {
            error!("Cause: {c}");
            cause = c.source();
        }
        process::exit(1);
    }
}

async fn fulfillment(
    State(state): State<AppState>,
    user: User,
    Json(payload): Json<Request>,
) -> Result<Json<Response>, ApiError> {
    debug!(username = user.preferred_username, "{payload:#?}");
    let gc = GoogleHome::new(&user.preferred_username);
    let devices = state.device_manager.devices().await;
    let result = gc
        .handle_request(payload, &devices)
        .await
        .map_err(|err| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, err.into()))?;

    debug!(username = user.preferred_username, "{result:#?}");

    Ok(Json(result))
}

async fn app() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    let config: Config = ::config::Config::builder()
        .add_source(
            File::with_name(&format!("{}.toml", std::env!("CARGO_PKG_NAME"))).required(false),
        )
        .add_source(
            Environment::default()
                .prefix(std::env!("CARGO_PKG_NAME"))
                .separator("__"),
        )
        .add_source(EnvironmentSecretFile::default())
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();

    info!("Starting automation_rs...");

    // Setup the device handler
    let device_manager = DeviceManager::new().await;

    let lua = mlua::Lua::new();

    lua.set_warning_function(|_lua, text, _cont| {
        warn!("{text}");
        Ok(())
    });
    let print = lua.create_function(|lua, values: mlua::Variadic<mlua::Value>| {
        // Fortmat the values the same way lua does by default
        let text: String = values
            .iter()
            .map(|value| {
                value.to_string().unwrap_or_else(|_| {
                    format!("{}: {}", value.type_name(), value.to_pointer().addr())
                })
            })
            .intersperse("\t".to_owned())
            .collect();

        // Level 1 of the stack gives us the location that called this function
        let (file, line) = lua
            .inspect_stack(1, |debug| {
                (
                    debug
                        .source()
                        .short_src
                        .unwrap_or("???".into())
                        .into_owned(),
                    debug.current_line().unwrap_or(0),
                )
            })
            .unwrap();

        // The target is overridden to make it possible to filter for logs originating from the
        // config
        info!(target: "automation_config", %file, line, "{text}");

        Ok(())
    })?;
    lua.globals().set("print", print)?;

    let mqtt = lua.create_table()?;
    let event_channel = device_manager.event_channel();
    let mqtt_new = lua.create_function(move |lua, config: mlua::Value| {
        let config: MqttConfig = lua.from_value(config)?;

        // Create a mqtt client
        // TODO: When starting up, the devices are not yet created, this could lead to a device being out of sync
        let (client, eventloop) = AsyncClient::new(config.into(), 100);
        mqtt::start(eventloop, &event_channel);

        Ok(WrappedAsyncClient(client))
    })?;
    mqtt.set("new", mqtt_new)?;
    lua.register_module("mqtt", mqtt)?;

    lua.register_module("device_manager", device_manager.clone())?;

    lua.register_module("variables", lua.to_value(&config.variables)?)?;
    lua.register_module("secrets", lua.to_value(&config.secrets)?)?;

    let utils = lua.create_table()?;
    let get_hostname = lua.create_function(|_lua, ()| {
        hostname::get()
            .map(|name| name.to_str().unwrap_or("unknown").to_owned())
            .map_err(mlua::ExternalError::into_lua_err)
    })?;
    utils.set("get_hostname", get_hostname)?;
    let get_epoch = lua.create_function(|_lua, ()| {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time is after UNIX EPOCH")
            .as_millis())
    })?;
    utils.set("get_epoch", get_epoch)?;
    lua.register_module("utils", utils)?;

    automation_devices::register_with_lua(&lua)?;
    helpers::register_with_lua(&lua)?;

    let entrypoint = Path::new(&config.entrypoint);
    let fulfillment_config: mlua::Value = lua.load(entrypoint).eval_async().await?;
    let fulfillment_config: FulfillmentConfig = lua.from_value(fulfillment_config)?;

    // Create google home fulfillment route
    let fulfillment = Router::new().route("/google_home", post(fulfillment));

    // Combine together all the routes
    let app = Router::new()
        .nest("/fulfillment", fulfillment)
        .with_state(AppState {
            openid_url: fulfillment_config.openid_url.clone(),
            device_manager,
        });

    // Start the web server
    let addr: SocketAddr = fulfillment_config.into();
    info!("Server started on http://{addr}");
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
