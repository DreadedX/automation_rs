#![feature(async_closure)]
use std::process;

use automation::auth::{OpenIDConfig, User};
use automation::config::Config;
use automation::device_manager::DeviceManager;
use automation::devices::{Ntfy, Presence};
use automation::error::ApiError;
use automation::mqtt;
use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use dotenvy::dotenv;
use google_home::{GoogleHome, Request};
use rumqttc::AsyncClient;
use tracing::{debug, error, info};

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
        std::env::var("AUTOMATION_CONFIG").unwrap_or("./config/config.yml".into());
    let config = Config::parse_file(&config_filename)?;

    // Create a mqtt client
    // TODO: Since we wait with starting the eventloop we might fill the queue while setting up devices
    let (client, eventloop) = AsyncClient::new(config.mqtt.clone(), 100);

    // Setup the device handler
    let device_manager = DeviceManager::new(client.clone());

    for (id, device_config) in config.devices {
        device_manager.create(&id, device_config).await?;
    }

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
