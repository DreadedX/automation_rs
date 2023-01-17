#![feature(async_closure)]
use std::{process, time::Duration};

use axum::{extract::FromRef, http::StatusCode, routing::post, Json, Router, response::IntoResponse};

use automation::{
    auth::User,
    config::{Config, OpenIDConfig},
    devices,
    hue_bridge,
    light_sensor, mqtt::Mqtt,
    ntfy,
    presence, error::ApiError, debug_bridge,
};
use dotenvy::dotenv;
use rumqttc::{AsyncClient, MqttOptions, Transport};
use tracing::{debug, error, info, metadata::LevelFilter};
use futures::future::join_all;

use google_home::{GoogleHome, Request};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    pub openid: OpenIDConfig,
}

impl FromRef<AppState> for automation::config::OpenIDConfig {
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


async fn app() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!("Starting automation_rs...");

    let config = std::env::var("AUTOMATION_CONFIG").unwrap_or("./config/config.toml".to_owned());
    let config = Config::parse_file(&config)?;

    // Configure MQTT
    let mqtt = config.mqtt.clone();
    let mut mqttoptions = MqttOptions::new(mqtt.client_name, mqtt.host, mqtt.port);
    mqttoptions.set_credentials(mqtt.username, mqtt.password);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    if mqtt.tls {
        mqttoptions.set_transport(Transport::tls_with_default_config());
    }

    // Create a mqtt client and wrap the eventloop
    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
    let mqtt = Mqtt::new(eventloop);
    let presence = presence::start(config.presence.clone(), mqtt.subscribe(), client.clone()).await?;
    let light_sensor = light_sensor::start(mqtt.subscribe(), config.light_sensor.clone(), client.clone()).await?;

    let devices = devices::start(mqtt.subscribe(), presence.clone(), light_sensor.clone());
    join_all(
        config
            .devices
            .clone()
            .into_iter()
            .map(|(identifier, device_config)| async {
                // Force the async block to move identifier
                let identifier = identifier;
                let device = device_config.create(&identifier, &config, client.clone()).await?;
                devices.add_device(device).await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            })
    ).await.into_iter().collect::<Result<_, _>>()?;

    // Start the ntfy service if it is configured
    if let Some(ntfy_config) = config.ntfy {
        ntfy::start(presence.clone(), &ntfy_config);
    }

    // Start the hue bridge if it is configured
    if let Some(hue_bridge_config) = config.hue_bridge {
        hue_bridge::start(presence.clone(), light_sensor.clone(), hue_bridge_config);
    }

    // Start the debug bridge if it is configured
    if let Some(debug_bridge_config) = config.debug_bridge {
        debug_bridge::start(presence.clone(), light_sensor.clone(), debug_bridge_config, client.clone());
    }

    // Actually start listening for mqtt message,
    // we wait until all the setup is done, as otherwise we might miss some messages
    mqtt.start();

    // Create google home fullfillment route
    let fullfillment = Router::new().route(
        "/google_home",
        post(async move |user: User, Json(payload): Json<Request>| {
            debug!(username = user.preferred_username, "{payload:#?}");
            let gc = GoogleHome::new(&user.preferred_username);
            let result = match devices.fullfillment(gc, payload).await {
                Ok(result) => result,
                Err(err) => return ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, err.into()).into_response(),
            };

            debug!(username = user.preferred_username, "{result:#?}");

            return (StatusCode::OK, Json(result)).into_response();
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
