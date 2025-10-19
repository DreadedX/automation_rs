#![feature(iter_intersperse)]

use std::net::SocketAddr;
use std::path::Path;
use std::process;

use ::config::{Environment, File};
use automation::config::{Config, Setup};
use automation::schedule::start_scheduler;
use automation::secret::EnvironmentSecretFile;
use automation::version::VERSION;
use automation::web::{ApiError, User};
use automation_lib::device_manager::DeviceManager;
use automation_lib::mqtt;
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use google_home::{GoogleHome, Request, Response};
use mlua::LuaSerdeExt;
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};

// Force automation_devices to link so that it gets registered as a module
extern crate automation_devices;

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
    tracing_subscriber::fmt::init();

    info!(version = VERSION, "automation_rs");

    let setup: Setup = ::config::Config::builder()
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

    automation_lib::load_modules(&lua)?;

    lua.register_module("automation:variables", lua.to_value(&setup.variables)?)?;
    lua.register_module("automation:secrets", lua.to_value(&setup.secrets)?)?;

    let entrypoint = Path::new(&setup.entrypoint);
    let config: Config = lua.load(entrypoint).eval_async().await?;

    let mqtt_client = mqtt::start(config.mqtt, &device_manager.event_channel());

    if let Some(devices) = config.devices {
        for device in devices.get(&lua, &mqtt_client).await? {
            device_manager.add(device).await;
        }
    }

    start_scheduler(config.schedule).await?;

    // Create google home fulfillment route
    let fulfillment = Router::new().route("/google_home", post(fulfillment));

    // Combine together all the routes
    let app = Router::new()
        .nest("/fulfillment", fulfillment)
        .with_state(AppState {
            openid_url: config.fulfillment.openid_url.clone(),
            device_manager,
        });

    // Start the web server
    let addr: SocketAddr = config.fulfillment.into();
    info!("Server started on http://{addr}");
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
