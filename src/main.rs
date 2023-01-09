#![feature(async_closure)]
use std::{time::Duration, sync::Arc, process};
use parking_lot::RwLock;

use axum::{Router, Json, routing::post, http::StatusCode, extract::FromRef};

use automation::{config::{Config, OpenIDConfig}, presence::Presence, ntfy::Ntfy, light_sensor::LightSensor, hue_bridge::HueBridge, auth::User};
use dotenvy::dotenv;
use rumqttc::{MqttOptions, Transport, AsyncClient};
use tracing::{error, info, metadata::LevelFilter};

use automation::{devices::Devices, mqtt::Mqtt};
use google_home::{GoogleHome, Request};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    pub openid: OpenIDConfig
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

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

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
    let mut mqtt = Mqtt::new(eventloop);

    // Create device holder and register it as listener for mqtt
    let devices = Arc::new(RwLock::new(Devices::new()));
    mqtt.add_listener(Arc::downgrade(&devices));

    // Turn the config into actual devices and add them
    config.devices.clone()
        .into_iter()
        .map(|(identifier, device_config)| {
            // This can technically block, but this only happens during start-up, so should not be
            // a problem
            device_config.into(identifier, &config, client.clone())
        })
        .for_each(|device| {
            devices.write().add_device(device);
        });

    // Setup presence system
    let mut presence = Presence::new(config.presence, client.clone());
    // Register devices as presence listener
    presence.add_listener(Arc::downgrade(&devices));

    let mut light_sensor = LightSensor::new(config.light_sensor, client.clone());
    light_sensor.add_listener(Arc::downgrade(&devices));

    let ntfy;
    if let Some(ntfy_config) = config.ntfy {
        ntfy = Arc::new(RwLock::new(Ntfy::new(ntfy_config)));
        presence.add_listener(Arc::downgrade(&ntfy));
    }

    let hue_bridge;
    if let Some(hue_bridge_config) = config.hue_bridge {
        hue_bridge = Arc::new(RwLock::new(HueBridge::new(hue_bridge_config)));
        presence.add_listener(Arc::downgrade(&hue_bridge));
        light_sensor.add_listener(Arc::downgrade(&hue_bridge));
    }

    // Register presence as mqtt listener
    let presence = Arc::new(RwLock::new(presence));
    mqtt.add_listener(Arc::downgrade(&presence));

    let light_sensor = Arc::new(RwLock::new(light_sensor));
    mqtt.add_listener(Arc::downgrade(&light_sensor));

    // Start mqtt, this spawns a seperate async task
    mqtt.start();

    // Create google home fullfillment route
    let fullfillment = Router::new()
        .route("/google_home", post(async move |user: User, Json(payload): Json<Request>| {
            // Handle request might block, so we need to spawn a blocking task
            tokio::task::spawn_blocking(move || {
                let gc = GoogleHome::new(&user.preferred_username);
                let result = gc.handle_request(payload, &mut devices.write().as_google_home_devices()).unwrap();

                return (StatusCode::OK, Json(result));
            }).await.unwrap()
        }));

    // Combine together all the routes
    let app = Router::new()
        .nest("/fullfillment", fullfillment)
        .with_state(AppState {
            openid: config.openid
        });

    // Start the web server
    let addr = config.fullfillment.into();
    info!("Server started on http://{addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
