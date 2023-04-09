#![feature(async_closure)]
use std::{collections::HashMap, process, time::Duration};

use axum::{
    extract::FromRef, http::StatusCode, response::IntoResponse, routing::post, Json, Router,
};

use automation::{
    auth::User,
    config::{Config, OpenIDConfig},
    debug_bridge, devices,
    error::ApiError,
    hue_bridge, light_sensor,
    mqtt::Mqtt,
    ntfy, presence,
};
use dotenvy::dotenv;
use futures::future::join_all;
use rumqttc::{matches, AsyncClient, MqttOptions, Transport};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, metadata::LevelFilter};

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

async fn app() -> anyhow::Result<()> {
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
    let presence =
        presence::start(config.presence.clone(), mqtt.subscribe(), client.clone()).await?;
    let light_sensor = light_sensor::start(
        mqtt.subscribe(),
        config.light_sensor.clone(),
        client.clone(),
    )
    .await?;

    // Start the ntfy service if it is configured
    let mut ntfy = None;
    if let Some(config) = &config.ntfy {
        ntfy = Some(ntfy::start(presence.clone(), config));
    }
    let ntfy = ntfy;

    // Start the hue bridge if it is configured
    if let Some(config) = &config.hue_bridge {
        hue_bridge::start(presence.clone(), light_sensor.clone(), config);
    }

    // Start the debug bridge if it is configured
    if let Some(config) = &config.debug_bridge {
        debug_bridge::start(
            presence.clone(),
            light_sensor.clone(),
            config,
            client.clone(),
        );
    }

    // Super hacky implementation for the washing machine, just for testing
    {
        let mut handle = None::<JoinHandle<()>>;
        let mut mqtt = mqtt.subscribe();
        client
            .subscribe("zigbee2mqtt/bathroom/washing", rumqttc::QoS::AtLeastOnce)
            .await
            .unwrap();
        tokio::spawn(async move {
            if let Some(ntfy) = ntfy {
                loop {
                    let message = mqtt.recv().await.unwrap();

                    if !matches(&message.topic, "zigbee2mqtt/bathroom/washing") {
                        continue;
                    }

                    let map: HashMap<String, serde_json::Value> =
                        serde_json::from_slice(&message.payload).unwrap();
                    debug!("Test: {:?}", map);

                    let strength = match map.get("strength").map(|value| value.as_i64().unwrap()) {
                        Some(s) => s,
                        None => continue,
                    };

                    if strength > 15 {
                        debug!("Strength over 15");

                        // Update of strength over 15 which means we are still running, cancel any
                        // running timer
                        if let Some(handle) = handle.take() {
                            handle.abort();
                        }

                        // Start a new timer, if the timer runs out we have not had an update of
                        // more then 15 in the last timeout period, assume we are done, notify user
                        let ntfy = ntfy.clone();
                        handle = Some(tokio::spawn(async move {
                            debug!("Starting timeout of 10 min for washing machine...");
                            tokio::time::sleep(Duration::from_secs(10 * 60)).await;
                            debug!("Notifying user!");

                            let notification = ntfy::Notification::new()
                                .set_title("Laundy is done")
                                .set_message("Do not forget to hang it!")
                                .set_priority(ntfy::Priority::High);

                            ntfy.send(notification).await.ok();
                        }));
                    }
                }
            }
        });
    }

    let devices = devices::start(mqtt.subscribe(), presence.clone(), light_sensor.clone());
    join_all(
        config
            .devices
            .clone()
            .into_iter()
            .map(|(identifier, device_config)| async {
                // Force the async block to move identifier
                let identifier = identifier;
                let device = device_config
                    .create(&identifier, &config, client.clone())
                    .await?;
                devices.add_device(device).await?;
                // We don't need a seperate error type in main
                anyhow::Ok(())
            }),
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
            let result = match devices.fullfillment(gc, payload).await {
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
