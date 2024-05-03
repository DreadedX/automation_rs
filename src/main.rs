#![feature(async_closure)]
use std::path::Path;
use std::process;

use anyhow::anyhow;
use automation::auth::User;
use automation::config::{FulfillmentConfig, MqttConfig};
use automation::device_manager::DeviceManager;
use automation::error::ApiError;
use automation::mqtt::{self, WrappedAsyncClient};
use automation::{devices, LUA};
use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use dotenvy::dotenv;
use google_home::{GoogleHome, Request};
use mlua::LuaSerdeExt;
use rumqttc::AsyncClient;
use tracing::{debug, error, info, warn};

#[derive(Clone)]
struct AppState {
    pub openid_url: String,
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

async fn app() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt::init();
    // console_subscriber::init();

    info!("Starting automation_rs...");

    // Setup the device handler
    let device_manager = DeviceManager::new().await;

    let fulfillment_config = {
        let lua = LUA.lock().await;

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

        devices::register_with_lua(&lua)?;

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
    let fulfillment = Router::new().route(
        "/google_home",
        post(async move |user: User, Json(payload): Json<Request>| {
            debug!(username = user.preferred_username, "{payload:#?}");
            let gc = GoogleHome::new(&user.preferred_username);
            let devices = device_manager.devices().await;
            let result = match gc.handle_request(payload, &devices).await {
                Ok(result) => result,
                Err(err) => {
                    return ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, err.into())
                        .into_response()
                }
            };

            debug!(username = user.preferred_username, "{result:#?}");

            (StatusCode::OK, Json(result)).into_response()
        }),
    );

    // Combine together all the routes
    let app = Router::new()
        .nest("/fulfillment", fulfillment)
        .with_state(AppState {
            openid_url: fulfillment_config.openid_url.clone(),
        });

    // Start the web server
    let addr = fulfillment_config.into();
    info!("Server started on http://{addr}");
    axum::Server::try_bind(&addr)?
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
