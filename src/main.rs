#![feature(async_closure)]
use std::{fs, process};

use automation::auth::{OpenIDConfig, User};
use automation::config::Config;
use automation::device_manager::DeviceManager;
use automation::devices::{
    AirFilter, AudioSetup, ContactSensor, DebugBridge, HueBridge, HueGroup, IkeaOutlet, KasaOutlet,
    LightSensor, Ntfy, Presence, WakeOnLAN, Washer,
};
use automation::error::ApiError;
use automation::mqtt::{self, WrappedAsyncClient};
use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use dotenvy::dotenv;
use google_home::{GoogleHome, Request};
use rumqttc::AsyncClient;
use tracing::{debug, error, info, warn};

#[derive(Clone)]
struct AppState {
    pub openid: OpenIDConfig,
}

impl FromRef<AppState> for OpenIDConfig {
    fn from_ref(input: &AppState) -> Self {
        input.openid.clone()
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

    let config_filename =
        std::env::var("AUTOMATION_CONFIG").unwrap_or("./config/config.yml".into());
    let config = Config::parse_file(&config_filename)?;

    // Create a mqtt client
    // TODO: Since we wait with starting the eventloop we might fill the queue while setting up devices
    let (client, eventloop) = AsyncClient::new(config.mqtt.clone(), 100);

    // Setup the device handler
    let device_manager = DeviceManager::new(client.clone());

    device_manager.add_schedule(config.schedule).await;

    let event_channel = device_manager.event_channel();

    // Create and add the presence system
    {
        let presence = Presence::new(config.presence, &event_channel);
        device_manager.add(Box::new(presence)).await;
    }

    // Start the ntfy service if it is configured
    if let Some(config) = config.ntfy {
        let ntfy = Ntfy::new(config, &event_channel);
        device_manager.add(Box::new(ntfy)).await;
    }

    // Lua testing
    {
        let lua = mlua::Lua::new();

        lua.set_warning_function(|_lua, text, _cont| {
            warn!("{text}");
            Ok(())
        });

        let automation = lua.create_table()?;

        automation.set("device_manager", device_manager.clone())?;
        automation.set("mqtt_client", WrappedAsyncClient(client.clone()))?;
        automation.set("event_channel", device_manager.event_channel())?;

        let util = lua.create_table()?;
        let get_env = lua.create_function(|_lua, name: String| {
            std::env::var(name).map_err(mlua::ExternalError::into_lua_err)
        })?;
        util.set("get_env", get_env)?;
        automation.set("util", util)?;

        lua.globals().set("automation", automation)?;

        // Register all the device types
        AirFilter::register_with_lua(&lua)?;
        AudioSetup::register_with_lua(&lua)?;
        ContactSensor::register_with_lua(&lua)?;
        DebugBridge::register_with_lua(&lua)?;
        HueBridge::register_with_lua(&lua)?;
        HueGroup::register_with_lua(&lua)?;
        IkeaOutlet::register_with_lua(&lua)?;
        KasaOutlet::register_with_lua(&lua)?;
        LightSensor::register_with_lua(&lua)?;
        WakeOnLAN::register_with_lua(&lua)?;
        Washer::register_with_lua(&lua)?;

        // TODO: Make this not hardcoded
        let filename = "config.lua";
        let file = fs::read_to_string(filename)?;
        match lua.load(file).set_name(filename).exec_async().await {
            Err(error) => {
                println!("{error}");
                Err(error)
            }
            result => result,
        }?;
    }

    // Wrap the mqtt eventloop and start listening for message
    // NOTE: We wait until all the setup is done, as otherwise we might miss some messages
    mqtt::start(eventloop, &event_channel);

    // Create google home fullfillment route
    let fullfillment = Router::new().route(
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
        .nest("/fullfillment", fullfillment)
        .with_state(AppState {
            openid: config.openid,
        });

    // Start the web server
    let addr = config.fullfillment.into();
    info!("Server started on http://{addr}");
    axum::Server::try_bind(&addr)?
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
