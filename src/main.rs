use std::{time::Duration, rc::Rc, cell::RefCell, process::exit};

use dotenv::dotenv;

use automation::{devices::{Devices, IkeaOutlet}, zigbee::Zigbee, mqtt::Notifier};
use rumqttc::{MqttOptions, Transport, Client};

fn get_required_env(name: &str) -> String {
    match std::env::var(name) {
        Ok(value) => value,
        _ => {
            eprintln!("Environment variable ${name} is not set!");
            exit(-1);
        }
    }
}

fn main() {
    dotenv().ok();

    // Setup MQTT
    let mut mqttoptions = MqttOptions::new("rust-test", get_required_env("MQTT_HOST"), 8883);
    mqttoptions.set_credentials(get_required_env("MQTT_USERNAME"), get_required_env("MQTT_PASSWORD"));
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_transport(Transport::tls_with_default_config());

    let (client, connection) = Client::new(mqttoptions, 10);

    // Create device holder
    let devices = Rc::new(RefCell::new(Devices::new()));

    // Create a new device and add it to the holder
    devices.borrow_mut().add_device(IkeaOutlet::new(Zigbee::new("kitchen/kettle", "zigbee2mqtt/kitchen/kettle"), client.clone()));

    let mut notifier = Notifier::new();

    {
        let mut temp = devices.borrow_mut();
        let a = temp.get_device(0);
        a.unwrap().as_state_on_off().unwrap().set_state(false);
    }

    notifier.add_listener(Rc::downgrade(&devices));

    notifier.start(connection);
}
