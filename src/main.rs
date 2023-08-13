#![feature(async_closure)]
use std::process;

use axum::{
    extract::FromRef, http::StatusCode, response::IntoResponse, routing::post, Json, Router,
};
use dotenvy::dotenv;
use rumqttc::AsyncClient;
use tracing::{debug, error, info};

use automation::{
    auth::{OpenIDConfig, User},
    config::Config,
    device_manager::DeviceManager,
    devices::{DebugBridge, HueBridge, LightSensor, Ntfy, Presence},
    error::ApiError,
    mqtt,
};
use google_home::{GoogleHome, Request};

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

    console_subscriber::init();

    info!("Starting automation_rs...");

    let config_filename =
        std::env::var("AUTOMATION_CONFIG").unwrap_or("./config/config.toml".to_owned());
    let config = Config::parse_file(&config_filename)?;

    // Create a mqtt client
    let (client, eventloop) = AsyncClient::new(config.mqtt.clone(), 10);

    // Setup the device handler
    let device_manager = DeviceManager::new(client.clone());
    let event_channel = device_manager.start();

    // Create all the devices specified in the config
    for (id, device_config) in config.devices {
        let device = device_config
            .create(
                &id,
                &event_channel,
                &client,
                &config.presence.mqtt.topic,
                &device_manager,
            )
            .await?;

        device_manager.add(device).await;
    }

    // Create and add the light sensor
    {
        let light_sensor = LightSensor::new(config.light_sensor, &event_channel);
        device_manager.add(Box::new(light_sensor)).await;
    }

    // Create and add the presence system
    {
        let presence = Presence::new(config.presence, &event_channel);
        device_manager.add(Box::new(presence)).await;
    }

    // If configured, create and add the hue bridge
    if let Some(config) = config.hue_bridge {
        let hue_bridge = HueBridge::new(config);
        device_manager.add(Box::new(hue_bridge)).await;
    }

    // Start the debug bridge if it is configured
    if let Some(config) = config.debug_bridge {
        let debug_bridge = DebugBridge::new(config, &client)?;
        device_manager.add(Box::new(debug_bridge)).await;
    }

    // Start the ntfy service if it is configured
    if let Some(config) = config.ntfy {
        let ntfy = Ntfy::new(config, &event_channel);
        device_manager.add(Box::new(ntfy)).await;
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
