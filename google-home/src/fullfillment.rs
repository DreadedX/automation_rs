use std::collections::HashMap;

use crate::{request::{Request, Intent, self}, device::GoogleHomeDevice, response::{sync, ResponsePayload, query, execute, Response, self, State}, errors::{DeviceError, ErrorCode}};

#[derive(Debug)]
pub struct GoogleHome {
    user_id: String,
    // Add credentials so we can notify google home of actions
}

impl GoogleHome {
    pub fn new(user_id: &str) -> Self {
        Self { user_id: user_id.into() }
    }

    pub fn handle_request(&self, request: Request, mut devices: &mut HashMap<&str, &mut dyn GoogleHomeDevice>) -> Result<Response, anyhow::Error> {
        // @TODO What do we do if we actually get more then one thing in the input array, right now
        // we only respond to the first thing
        let payload = request
            .inputs
            .into_iter()
            .map(|input| match input {
                Intent::Sync => ResponsePayload::Sync(self.sync(&devices)),
                Intent::Query(payload) => ResponsePayload::Query(self.query(payload, &devices)),
                Intent::Execute(payload) => ResponsePayload::Execute(self.execute(payload, &mut devices)),
            }).next();

        match payload {
            Some(payload) => Ok(Response::new(&request.request_id, payload)),
            _ => Err(anyhow::anyhow!("Expected at least one ResponsePayload")),
        }
    }

    fn sync(&self, devices: &HashMap<&str, &mut dyn GoogleHomeDevice>) -> sync::Payload {
        let mut resp_payload = sync::Payload::new(&self.user_id);
        resp_payload.devices = devices
            .iter()
            .map(|(_, device)| device.sync())
            .collect::<Vec<_>>();

        return resp_payload;
    }

    fn query(&self, payload: request::query::Payload, devices: &HashMap<&str, &mut dyn GoogleHomeDevice>) -> query::Payload {
        let mut resp_payload = query::Payload::new();
        resp_payload.devices = payload.devices
            .into_iter()
            .map(|device| device.id)
            .map(|id| {
                let device = devices.get(id.as_str())
                    .map_or_else(|| {
                        let mut device = query::Device::new();
                        device.set_offline();
                        device.set_error(DeviceError::DeviceNotFound.into());

                        device
                    }, |device| device.query());

                return (id, device);
            }).collect();

        return resp_payload;

    }

    fn execute(&self, payload: request::execute::Payload, devices: &mut HashMap<&str, &mut dyn GoogleHomeDevice>) -> execute::Payload {
        let mut resp_payload = response::execute::Payload::new();

        payload.commands
            .into_iter()
            .for_each(|command| {
                let mut success = response::execute::Command::new(execute::Status::Success);
                success.states = Some(execute::States { online: true, state: State::default() });
                let mut offline = response::execute::Command::new(execute::Status::Offline);
                offline.states = Some(execute::States { online: false, state: State::default() });
                let mut errors: HashMap<ErrorCode, response::execute::Command> = HashMap::new();

                command.devices
                    .into_iter()
                    .map(|device| device.id)
                    .map(|id| {
                        devices.get_mut(id.as_str())
                            .map_or((id.clone(), Err(DeviceError::DeviceNotFound.into())), |device| {
                                if !device.is_online() {
                                    return (id, Ok(false));
                                }

                                let results = command.execution.iter().map(|cmd| {
                                    // @TODO We should also return the state after update in the state
                                    // struct, however that will make things WAY more complicated
                                    device.execute(cmd)
                                }).collect::<Result<Vec<_>, ErrorCode>>();

                                // @TODO We only get one error not all errors
                                if let Err(err) = results {
                                    return (id, Err(err));
                                } else {
                                    return (id, Ok(true));
                                }
                            })
                    }).for_each(|(id, state)| {
                        match state {
                            Ok(true) => success.add_id(&id),
                            Ok(false) => offline.add_id(&id),
                            Err(err) => errors.entry(err).or_insert_with(|| {
                                match &err {
                                    ErrorCode::DeviceError(_) => response::execute::Command::new(execute::Status::Error),
                                    ErrorCode::DeviceException(_) => response::execute::Command::new(execute::Status::Exceptions),
                                }
                            }).add_id(&id),
                        };
                    });

                resp_payload.add_command(success);
                resp_payload.add_command(offline);
                for (error, mut cmd) in errors {
                    cmd.error_code = Some(error);
                    resp_payload.add_command(cmd);
                }
            });

        return resp_payload;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{request::Request, device::{GoogleHomeDevice, self}, types, traits, errors::ErrorCode};

    #[derive(Debug)]
    struct TestOutlet {
        name: String,
        on: bool,
    }

    impl TestOutlet {
        fn new(name: &str) -> Self {
            Self { name: name.into(), on: false }
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
            return &self.name;
        }

        fn is_online(&self) -> bool {
            true
        }

        fn get_room_hint(&self) -> Option<&str> {
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
        fn is_on(&self) -> Result<bool, ErrorCode> {
            Ok(self.on)
        }

        fn set_on(&mut self, on: bool) -> Result<(), ErrorCode> {
            self.on = on;
            Ok(())
        }
    }

    #[derive(Debug)]
    struct TestScene;

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

        fn is_online(&self) -> bool {
            true
        }

        fn get_room_hint(&self) -> Option<&str> {
            Some("Living room")
        }
    }

    impl traits::Scene for TestScene {
        fn set_active(&self, _activate: bool) -> Result<(), ErrorCode> {
            println!("Activating the party scene");
            Ok(())
        }
    }

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

        let mut nightstand = TestOutlet::new("bedroom/nightstand");
        let mut lamp = TestOutlet::new("living/lamp");
        let mut scene = TestScene::new();
        let mut devices: HashMap<&str, &mut dyn GoogleHomeDevice> = HashMap::new();
        let id = nightstand.get_id().to_owned();
        devices.insert(&id, &mut nightstand);
        let id = lamp.get_id().to_owned();
        devices.insert(&id, &mut lamp);
        let id = scene.get_id().to_owned();
        devices.insert(&id, &mut scene);

        let resp = gh.handle_request(req, &mut devices).unwrap();

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

        let mut nightstand = TestOutlet::new("bedroom/nightstand");
        let mut lamp = TestOutlet::new("living/lamp");
        let mut scene = TestScene::new();
        let mut devices: HashMap<&str, &mut dyn GoogleHomeDevice> = HashMap::new();
        let id = nightstand.get_id().to_owned();
        devices.insert(&id, &mut nightstand);
        let id = lamp.get_id().to_owned();
        devices.insert(&id, &mut lamp);
        let id = scene.get_id().to_owned();
        devices.insert(&id, &mut scene);

        let resp = gh.handle_request(req, &mut devices).unwrap();

        let json = serde_json::to_string(&resp).unwrap();
        println!("{}", json)
    }

    #[test]
    fn handle_execute() {
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
                "id": "bedroom/nightstand"
              },
              {
                "id": "living/lamp"
              }
            ],
            "execution": [
              {
                "command": "action.devices.commands.OnOff",
                "params": {
                  "on": true
                }
              }
            ]
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

        let mut nightstand = TestOutlet::new("bedroom/nightstand");
        let mut lamp = TestOutlet::new("living/lamp");
        let mut scene = TestScene::new();
        let mut devices: HashMap<&str, &mut dyn GoogleHomeDevice> = HashMap::new();
        let id = nightstand.get_id().to_owned();
        devices.insert(&id, &mut nightstand);
        let id = lamp.get_id().to_owned();
        devices.insert(&id, &mut lamp);
        let id = scene.get_id().to_owned();
        devices.insert(&id, &mut scene);

        let resp = gh.handle_request(req, &mut devices).unwrap();

        let json = serde_json::to_string(&resp).unwrap();
        println!("{}", json)
    }
}
