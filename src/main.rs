use std::{time::Duration, sync::{Arc, RwLock}, process, net::SocketAddr};

use automation::config::Config;
use dotenv::dotenv;
use warp::Filter;
use rumqttc::{MqttOptions, Transport, AsyncClient};
use env_logger::Builder;
use log::{error, info, debug, LevelFilter};

use automation::{devices::Devices, mqtt::Notifier};
use google_home::{GoogleHome, Request};

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Setup logger
    Builder::new()
        .filter_module("automation", LevelFilter::Info)
        .parse_default_env()
        .init();

    let config = std::env::var("AUTOMATION_CONFIG").unwrap_or("./config/config.toml".to_owned());
    let config = Config::build(&config).unwrap_or_else(|err| {
        error!("Failed to load config: {err}");
        process::exit(1);
    });

    info!("Starting automation_rs...");

    // Create device holder
    // @TODO Make this nices to work with, we devices.rs
    let devices = Arc::new(RwLock::new(Devices::new()));

    // Setup MQTT
    let mut mqttoptions = MqttOptions::new("rust-test", config.mqtt.host, config.mqtt.port);
    mqttoptions.set_credentials(config.mqtt.username, config.mqtt.password.unwrap());
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_transport(Transport::tls_with_default_config());

    // Create a notifier and start it in a seperate task
    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
    let mut notifier = Notifier::new(eventloop);
    notifier.add_listener(Arc::downgrade(&devices));
    notifier.start();

    // Create devices based on config
    // @TODO Move out of main (config? or maybe devices?)
    config.devices
        .into_iter()
        .map(|(identifier, device_config)| {
            device_config.into(identifier, client.clone())
        })
        .for_each(|device| {
            debug!("Adding device {}", device.get_id());
            devices.write().unwrap().add_device(device);
        });

    // Google Home fullfillments
    let fullfillment_google_home = warp::path("google_home")
        .and(warp::post())
        .and(warp::body::json())
        .map(move |request: Request| {
            // @TODO Verify that we are actually logged in
            // Might also be smart to get the username from here
            let gc = GoogleHome::new(&config.fullfillment.username);
            let result = gc.handle_request(request, &mut devices.write().unwrap().as_google_home_devices()).unwrap();

            warp::reply::json(&result)
        });

    // Combine all fullfillments together
    let fullfillment = warp::path("fullfillment").and(fullfillment_google_home);

    // Combine all routes together
    let routes = fullfillment;

    // Start the web server
    let addr: SocketAddr = ([127, 0, 0, 1], config.fullfillment.port).into();
    info!("Server started on http://{addr}");
    warp::serve(routes)
        .run(addr)
        .await;
}
