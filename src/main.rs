use std::{time::Duration, rc::Rc, cell::RefCell, process::exit};

use dotenv::dotenv;

use automation::{devices::{Devices, IkeaOutlet, TestOutlet}, zigbee::Zigbee, mqtt::Notifier};
use google_home::GoogleHome;
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
    devices.borrow_mut().add_device(IkeaOutlet::new("Kettle".into(), Zigbee::new("kitchen/kettle", "zigbee2mqtt/kitchen/kettle"), client.clone()));

    devices.borrow_mut().add_device(TestOutlet::new());

    {
        for (_, d) in devices.borrow_mut().as_on_offs().iter_mut() {
            d.set_on(true).unwrap();
        }
    }

    let gc = GoogleHome::new("Dreaded_X");

    let json = r#"{
  "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
  "inputs": [
    {
      "intent": "action.devices.EXECUTE",
      "payload": {
        "commands": [
          {
            "devices": [
              {
                "id": "kitchen/kettle"
              },
              {
                "id": "test_device"
              }
            ],
            "execution": [
              {
                "command": "action.devices.commands.OnOff",
                "params": {
                  "on": false
                }
              }
            ]
          }
        ]
      }
    }
  ]
}"#;
    let request = serde_json::from_str(json).unwrap();
    {
        let mut binding = devices.borrow_mut();
        let mut ghd = binding.as_fullfillments();

        let response = gc.handle_request(request, &mut ghd).unwrap();

        println!("{response:?}");
    }

    let mut notifier = Notifier::new();

    notifier.add_listener(Rc::downgrade(&devices));

    notifier.start(connection);
}
