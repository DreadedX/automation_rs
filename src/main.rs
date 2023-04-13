#![feature(async_closure)]
use std::process;

use axum::{
    extract::FromRef, http::StatusCode, response::IntoResponse, routing::post, Json, Router,
};

use automation::{
    auth::{OpenIDConfig, User},
    config::Config,
    debug_bridge, devices,
    error::ApiError,
    hue_bridge, light_sensor,
    mqtt::Mqtt,
    ntfy, presence,
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

    // Create a mqtt client and wrap the eventloop
    let (client, eventloop) = AsyncClient::new(config.mqtt.clone(), 10);
    let mqtt = Mqtt::new(eventloop);
    let presence =
        presence::start(config.presence.clone(), mqtt.subscribe(), client.clone()).await?;
    let light_sensor = light_sensor::start(
        mqtt.subscribe(),
        config.light_sensor.clone(),
        client.clone(),
    )
    .await?;

    // Start the ntfy service if it is configured
    if let Some(config) = &config.ntfy {
        ntfy::start(presence.clone(), config);
    }

    // Start the hue bridge if it is configured
    if let Some(config) = config.hue_bridge {
        hue_bridge::start(presence.clone(), light_sensor.clone(), config);
    }

    // Start the debug bridge if it is configured
    if let Some(config) = config.debug_bridge {
        debug_bridge::start(
            presence.clone(),
            light_sensor.clone(),
            config,
            client.clone(),
        );
    }

    // Setup the device handler
    let device_handler = devices::start(
        mqtt.subscribe(),
        presence.clone(),
        light_sensor.clone(),
        client.clone(),
    );

    // Create all the devices specified in the config
    let devices = config
        .devices
        .into_iter()
        .map(|(identifier, device_config)| {
            device_config.create(&identifier, client.clone(), &config.presence.topic)
        })
        .collect::<Result<Vec<_>, _>>()?;

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

    // Actually start listening for mqtt message,
    // we wait until all the setup is done, as otherwise we might miss some messages
    mqtt.start();

    // Create google home fullfillment route
    let fullfillment = Router::new().route(
        "/google_home",
        post(async move |user: User, Json(payload): Json<Request>| {
            debug!(username = user.preferred_username, "{payload:#?}");
            let gc = GoogleHome::new(&user.preferred_username);
            let result = match device_handler.fullfillment(gc, payload).await {
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
