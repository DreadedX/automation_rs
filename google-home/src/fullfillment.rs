use std::collections::HashMap;

use crate::{request::{Request, Intent, self}, device::Fullfillment, response::{sync, ResponsePayload, query, execute, Response}, errors::Errors};

pub struct GoogleHome {
    user_id: String,
    // Add credentials so we can notify google home of actions
}

impl GoogleHome {
    pub fn new(user_id: &str) -> Self {
        Self { user_id: user_id.into() }
    }

    pub fn handle_request(&self, request: Request, devices: &HashMap<String, &mut dyn Fullfillment>) -> Result<Response, anyhow::Error> {
        // @TODO What do we do if we actually get more then one thing in the input array, right now
        // we only respond to the first thing
        let payload = request
            .inputs
            .into_iter()
            .map(|input| match input {
                Intent::Sync => ResponsePayload::Sync(self.sync(&devices)),
                Intent::Query(payload) => ResponsePayload::Query(self.query(payload, &devices)),
                Intent::Execute(payload) => ResponsePayload::Execute(self.execute(payload, &devices)),
            }).next();

        match payload {
            Some(payload) => Ok(Response::new(request.request_id, payload)),
            _ => Err(anyhow::anyhow!("Something went wrong, expected at least ResponsePayload")),
        }
    }

    fn sync(&self, devices: &HashMap<String, &mut dyn Fullfillment>) -> sync::Payload {
        let mut resp_payload = sync::Payload::new(&self.user_id);
        resp_payload.devices = devices.iter().map(|(_, device)| device.sync()).collect::<Vec<_>>();

        return resp_payload;
    }

    fn query(&self, payload: request::query::Payload, devices: &HashMap<String, &mut dyn Fullfillment>) -> query::Payload {
        let mut resp_payload = query::Payload::new();
        for request::query::Device{id} in payload.devices {
            let mut d: query::Device;
            if let Some(device) = devices.get(&id) {
                d = device.query();
            } else {
                d = query::Device::new(false, query::Status::Error);
                d.error_code = Some(Errors::DeviceNotFound);
            }
            resp_payload.add_device(&id, d)
        }

        return resp_payload;

    }

    fn execute(&self, payload: request::execute::Payload, devices: &HashMap<String, &mut dyn Fullfillment>) -> execute::Payload {
        return execute::Payload::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{request::Request, device::{GoogleHomeDevice, self}, types, traits};

    struct TestOutlet {
        on: bool,
    }

    impl TestOutlet {
        fn new() -> Self {
            Self { on: false }
        }
    }

    impl<'a> GoogleHomeDevice<'a> for TestOutlet {
        fn get_device_type(&self) -> types::Type {
            types::Type::Outlet
        }

        fn get_device_name(&self) -> device::Name {
            let mut name = device::Name::new("Nightstand");
            name.add_default_name("Outlet");
            name.add_nickname("Nightlight");

            return name;
        }

        fn get_id(&self) -> &'a str {
            return "bedroom/nightstand";
        }

        fn is_online(&self) -> bool {
            true
        }

        fn get_room_hint(&self) -> Option<&'a str> {
            Some("Bedroom")
        }

        fn get_device_info(&self) -> Option<device::Info> {
            Some(device::Info {
                manufacturer: Some("Company".into()),
                model: Some("Outlet II".into()),
                hw_version: None,
                sw_version: None,
            })
        }
    }

    impl traits::OnOff for TestOutlet {
        fn is_on(&self) -> Result<bool, anyhow::Error> {
            Ok(self.on)
        }

        fn set_on(&mut self, on: bool) -> Result<(), anyhow::Error> {
            self.on = on;
            Ok(())
        }
    }

    impl traits::AsScene for TestOutlet {}


    struct TestScene {}

    impl TestScene {
        fn new() -> Self {
            Self {}
        }
    }

    impl<'a> GoogleHomeDevice<'a> for TestScene {
        fn get_device_type(&self) -> types::Type {
            types::Type::Scene
        }

        fn get_device_name(&self) -> device::Name {
            device::Name::new("Party")
        }

        fn get_id(&self) -> &'a str {
            return "living/party_mode";
        }

        fn is_online(&self) -> bool {
            true
        }

        fn get_room_hint(&self) -> Option<&'a str> {
            Some("Living room")
        }
    }

    impl traits::Scene for TestScene {
        fn set_active(&self, _activate: bool) -> Result<(), anyhow::Error> {
            println!("Activating the party scene");
            Ok(())
        }
    }

    impl traits::AsOnOff for TestScene {}



    #[test]
    fn handle_sync() {
        let json = r#"{
  "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
  "inputs": [
    {
      "intent": "action.devices.SYNC"
    }
  ]
}"#;
        let req: Request = serde_json::from_str(json).unwrap();

        let gh = GoogleHome {
            user_id: "Dreaded_X".into(),
        };

        let mut device = TestOutlet::new();
        let mut scene = TestScene::new();
        let mut devices: HashMap<String, &mut dyn Fullfillment> = HashMap::new();
        devices.insert(device.get_id().into(), &mut device);
        devices.insert(scene.get_id().into(), &mut scene);

        let resp = gh.handle_request(req, &devices).unwrap();

        let json = serde_json::to_string(&resp).unwrap();
        println!("{}", json)
    }

    #[test]
    fn handle_query() {
        let json = r#"{
  "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
  "inputs": [
    {
      "intent": "action.devices.QUERY",
      "payload": {
        "devices": [
          {
            "id": "bedroom/nightstand"
          },
          {
            "id": "living/party_mode"
          }
        ]
      }
    }
  ]
}"#;
        let req: Request = serde_json::from_str(json).unwrap();

        let gh = GoogleHome {
            user_id: "Dreaded_X".into(),
        };

        let mut device = TestOutlet::new();
        let mut scene = TestScene::new();
        let mut devices: HashMap<String, &mut dyn Fullfillment> = HashMap::new();
        devices.insert(device.get_id().into(), &mut device);
        devices.insert(scene.get_id().into(), &mut scene);

        let resp = gh.handle_request(req, &devices).unwrap();

        let json = serde_json::to_string(&resp).unwrap();
        println!("{}", json)
    }
}
