use std::{time::Duration, sync::{Arc, RwLock}, process::exit, net::SocketAddr};

use warp::Filter;
use rumqttc::{MqttOptions, Transport, AsyncClient};
use dotenv::dotenv;
use env_logger::Builder;
use log::{error, info, LevelFilter};

use automation::{devices::{Devices, IkeaOutlet, TestOutlet}, zigbee::Zigbee, mqtt::Notifier};
use google_home::{GoogleHome, Request};

fn get_required_env(name: &str) -> String {
    match std::env::var(name) {
        Ok(value) => value,
        _ => {
            error!("Environment variable ${name} is not set!");
            exit(-1);
        }
    }
}

#[tokio::main]
async fn main() {
    // Setup logger
    Builder::new()
        .filter_module("automation", LevelFilter::Info)
        .parse_default_env()
        .init();

    // Load dotfiles
    dotenv().ok();

    info!("Starting automation_rs...");

    // Create device holder
    // @TODO Make this nices to work with, we devices.rs
    let devices = Arc::new(RwLock::new(Devices::new()));

    // Setup MQTT
    let mut mqttoptions = MqttOptions::new("rust-test", get_required_env("MQTT_HOST"), 8883);
    mqttoptions.set_credentials(get_required_env("MQTT_USERNAME"), get_required_env("MQTT_PASSWORD"));
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_transport(Transport::tls_with_default_config());

    // Create a notifier and move it to a new thread
    // @TODO Maybe rename this to make it clear it has to do with mqtt
    let mut notifier = Notifier::new();
    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
    notifier.add_listener(Arc::downgrade(&devices));
    tokio::spawn(async move {
        info!("Connecting to MQTT broker");
        notifier.start(eventloop).await;
        todo!("Error in MQTT (most likely lost connection to mqtt server), we need to handle these errors!");
    });

    // @TODO Load these from a config
    // Create a new device and add it to the holder
    devices.write().unwrap().add_device(IkeaOutlet::new("Kettle".into(), Zigbee::new("kitchen/kettle", "zigbee2mqtt/kitchen/kettle"), client.clone()));
    devices.write().unwrap().add_device(TestOutlet::new());

    // Google Home fullfillments
    let fullfillment_google_home = warp::path("google_home")
        .and(warp::post())
        .and(warp::body::json())
        .map(move |request: Request| {
            // @TODO Verify that we are actually logged in
            // Might also be smart to get the username from here
            let gc = GoogleHome::new("Dreaded_X");
            let result = gc.handle_request(request, &mut devices.write().unwrap().as_google_home_devices()).unwrap();

            warp::reply::json(&result)
        });

    // Combine all fullfillments together
    let fullfillment = warp::path("fullfillment").and(fullfillment_google_home);

    // Combine all routes together
    let routes = fullfillment;

    // Start the web server
    let addr: SocketAddr = ([127, 0, 0, 1], 7878).into();
    info!("Server started on http://{addr}");
    warp::serve(routes)
        .run(addr)
        .await;
}
