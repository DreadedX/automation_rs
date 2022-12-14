use crate::{request::{Request, Intent, self}, device::GoogleHomeDeviceFullfillment, response::{sync, ResponsePayload, query, execute, Response}};

pub struct GoogleHome {
    user_id: String,
    // Add credentials so we can notify google home of actions
}

impl GoogleHome {
    pub fn new(user_id: &str) -> Self {
        Self { user_id: user_id.into() }
    }

    pub fn handle_request(&self, request: Request, devices: Vec<&mut dyn GoogleHomeDeviceFullfillment>) -> Result<Response, anyhow::Error> {
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

    fn sync(&self, devices: &Vec<&mut dyn GoogleHomeDeviceFullfillment>) -> sync::Payload {
        let mut payload = sync::Payload::new(&self.user_id);
        payload.devices = devices.iter().map(|device| device.sync()).collect::<Vec<_>>();

        return payload;
    }

    fn query(&self, payload: request::query::Payload, devices: &Vec<&mut dyn GoogleHomeDeviceFullfillment>) -> query::Payload {
        return query::Payload::new();
    }

    fn execute(&self, payload: request::execute::Payload, devices: &Vec<&mut dyn GoogleHomeDeviceFullfillment>) -> execute::Payload {
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

    impl GoogleHomeDevice for TestOutlet {
        fn get_device_type(&self) -> types::Type {
            types::Type::Outlet
        }

        fn get_device_name(&self) -> device::Name {
            let mut name = device::Name::new("Nightstand");
            name.add_default_name("Outlet");
            name.add_nickname("Nightlight");

            return name;
        }

        fn get_id(&self) -> &str {
            return "bedroom/nightstand";
        }

        fn get_room_hint(&self) -> Option<String> {
            Some("Bedroom".into())
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

    impl GoogleHomeDevice for TestScene {
        fn get_device_type(&self) -> types::Type {
            types::Type::Scene
        }

        fn get_device_name(&self) -> device::Name {
            device::Name::new("Party")
        }

        fn get_id(&self) -> &str {
            return "living/party_mode";
        }

        fn get_room_hint(&self) -> Option<String> {
            Some("Living room".into())
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
        let devices: Vec<&mut dyn GoogleHomeDeviceFullfillment> = vec![&mut device, &mut scene];

        let resp = gh.handle_request(req, devices).unwrap();

        let json = serde_json::to_string(&resp).unwrap();
        println!("{}", json)
    }
}
