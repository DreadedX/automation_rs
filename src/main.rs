#![feature(async_closure)]
use std::{process, time::Duration};

use axum::{extract::FromRef, http::StatusCode, routing::post, Json, Router};

use automation::{
    auth::User,
    config::{Config, OpenIDConfig},
    devices,
    hue_bridge::HueBridge,
    light_sensor, mqtt,
    ntfy::Ntfy,
    presence,
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
    dotenv().ok();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let config = std::env::var("AUTOMATION_CONFIG").unwrap_or("./config/config.toml".to_owned());
    let config = Config::build(&config).unwrap_or_else(|err| {
        error!("Failed to load config: {err}");
        process::exit(1);
    });

    info!("Starting automation_rs...");

    // Configure MQTT
    let mqtt = config.mqtt.clone();
    let mut mqttoptions = MqttOptions::new("rust-test", mqtt.host, mqtt.port);
    mqttoptions.set_credentials(mqtt.username, mqtt.password);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_transport(Transport::tls_with_default_config());

    // Create a mqtt client and wrap the eventloop
    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
    let mqtt = mqtt::start(eventloop);
    let presence = presence::start(mqtt.clone(), config.presence.clone(), client.clone()).await;
    let light_sensor = light_sensor::start(mqtt.clone(), config.light_sensor.clone(), client.clone()).await;

    let devices = devices::start(mqtt, presence.clone(), light_sensor.clone());
    join_all(
        config
            .devices
            .clone()
            .into_iter()
            .map(|(identifier, device_config)| async {
                // This can technically block, but this only happens during start-up, so should not be
                // a problem
                let device = device_config.into(identifier, &config, client.clone()).await;
                devices.add_device(device).await;
            })
    ).await;

    // Start the ntfy service if it is configured
    if let Some(ntfy_config) = config.ntfy {
        Ntfy::create(presence.clone(), ntfy_config);
    }

    // Start he hue bridge if it is configured
    if let Some(hue_bridge_config) = config.hue_bridge {
        HueBridge::create(presence.clone(), light_sensor.clone(), hue_bridge_config);
    }

    // Create google home fullfillment route
    let fullfillment = Router::new().route(
        "/google_home",
        post(async move |user: User, Json(payload): Json<Request>| {
            debug!(username = user.preferred_username, "{payload:?}");
            let gc = GoogleHome::new(&user.preferred_username);
            let result = devices.fullfillment(gc, payload).await.unwrap();

            debug!(username = user.preferred_username, "{result:?}");

            return (StatusCode::OK, Json(result));
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
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
