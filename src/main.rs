#![feature(async_closure)]
use std::process;

use axum::{
    extract::FromRef, http::StatusCode, response::IntoResponse, routing::post, Json, Router,
};

use automation::{
    auth::{OpenIDConfig, User},
    config::Config,
    devices,
    devices::{DebugBridge, HueBridge, LightSensor, Ntfy, Presence},
    error::ApiError,
    mqtt,
};
use dotenvy::dotenv;
use futures::future::join_all;
use rumqttc::AsyncClient;
use tracing::{debug, error, info};

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
    let (device_handler, event_channel) = devices::start(client.clone());

    // Create all the devices specified in the config
    let mut devices = config
        .devices
        .into_iter()
        .map(|(identifier, device_config)| {
            device_config.create(
                &identifier,
                &event_channel,
                &client,
                &config.presence.mqtt.topic,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Create and add the light sensor
    {
        let light_sensor = LightSensor::new(config.light_sensor, &event_channel);
        devices.push(Box::new(light_sensor));
    }

    // Create and add the presence system
    {
        let presence = Presence::new(config.presence, &event_channel);
        devices.push(Box::new(presence));
    }

    // If configured, create and add the hue bridge
    if let Some(config) = config.hue_bridge {
        let hue_bridge = HueBridge::new(config);
        devices.push(Box::new(hue_bridge));
    }

    // Start the debug bridge if it is configured
    if let Some(config) = config.debug_bridge {
        let debug_bridge = DebugBridge::new(config, &client)?;
        devices.push(Box::new(debug_bridge));
    }

    // Start the ntfy service if it is configured
    if let Some(config) = config.ntfy {
        let ntfy = Ntfy::new(config, &event_channel);
        devices.push(Box::new(ntfy));
    }

    // Can even add some more devices here
    // devices.push(device)

    // Register all the devices to the device_handler
    join_all(
        devices
            .into_iter()
            .map(|device| async { device_handler.add_device(device).await }),
    )
    .await
    .into_iter()
    .collect::<Result<_, _>>()?;

    // Wrap the mqtt eventloop and start listening for message
    // NOTE: We wait until all the setup is done, as otherwise we might miss some messages
    mqtt::start(eventloop, &event_channel);

    // Create google home fullfillment route
    let fullfillment = Router::new().route(
        "/google_home",
        post(async move |user: User, Json(payload): Json<Request>| {
            debug!(username = user.preferred_username, "{payload:#?}");
            let gc = GoogleHome::new(&user.preferred_username);
            let result = match device_handler.fullfillment().await {
                Ok(devices) => match gc.handle_request(payload, &devices).await {
                    Ok(result) => result,
                    Err(err) => {
                        return ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, err.into())
                            .into_response()
                    }
                },
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
