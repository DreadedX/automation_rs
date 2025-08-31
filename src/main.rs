mod web;

use std::net::SocketAddr;
use std::path::Path;
use std::process;

use anyhow::anyhow;
use automation_lib::config::{FulfillmentConfig, MqttConfig};
use automation_lib::device_manager::DeviceManager;
use automation_lib::helpers;
use automation_lib::mqtt::{self, WrappedAsyncClient};
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use dotenvy::dotenv;
use google_home::{GoogleHome, Request, Response};
use mlua::LuaSerdeExt;
use rumqttc::AsyncClient;
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};
use web::{ApiError, User};

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
    // console_subscriber::init();

    info!("Starting automation_rs...");

    // Setup the device handler
    let device_manager = DeviceManager::new().await;

    let fulfillment_config = {
        let lua = mlua::Lua::new();

        lua.set_warning_function(|_lua, text, _cont| {
            warn!("{text}");
            Ok(())
        });

        let automation = lua.create_table()?;
        let event_channel = device_manager.event_channel();
        let new_mqtt_client = lua.create_function(move |lua, config: mlua::Value| {
            let config: MqttConfig = lua.from_value(config)?;

            // Create a mqtt client
            // TODO: When starting up, the devices are not yet created, this could lead to a device being out of sync
            let (client, eventloop) = AsyncClient::new(config.into(), 100);
            mqtt::start(eventloop, &event_channel);

            Ok(WrappedAsyncClient(client))
        })?;

        automation.set("new_mqtt_client", new_mqtt_client)?;
        automation.set("device_manager", device_manager.clone())?;

        let util = lua.create_table()?;
        let get_env = lua.create_function(|_lua, name: String| {
            std::env::var(name).map_err(mlua::ExternalError::into_lua_err)
        })?;
        util.set("get_env", get_env)?;
        let get_hostname = lua.create_function(|_lua, ()| {
            hostname::get()
                .map(|name| name.to_str().unwrap_or("unknown").to_owned())
                .map_err(mlua::ExternalError::into_lua_err)
        })?;
        util.set("get_hostname", get_hostname)?;
        automation.set("util", util)?;

        lua.globals().set("automation", automation)?;

        automation_devices::register_with_lua(&lua)?;
        helpers::register_with_lua(&lua)?;

        // TODO: Make this not hardcoded
        let config_filename = std::env::var("AUTOMATION_CONFIG").unwrap_or("./config.lua".into());
        let config_path = Path::new(&config_filename);
        match lua.load(config_path).exec_async().await {
            Err(error) => {
                println!("{error}");
                Err(error)
            }
            result => result,
        }?;

        let automation: mlua::Table = lua.globals().get("automation")?;
        let fulfillment_config: Option<mlua::Value> = automation.get("fulfillment")?;
        if let Some(fulfillment_config) = fulfillment_config {
            let fulfillment_config: FulfillmentConfig = lua.from_value(fulfillment_config)?;
            debug!("automation.fulfillment = {fulfillment_config:?}");
            fulfillment_config
        } else {
            return Err(anyhow!("Fulfillment is not configured"));
        }
    };

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
