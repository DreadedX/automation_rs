use std::collections::HashMap;
use std::sync::Arc;

use futures::future::{join_all, OptionFuture};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};

use crate::device::AsGoogleHomeDevice;
use crate::errors::{DeviceError, ErrorCode};
use crate::request::{self, Intent, Request};
use crate::response::{self, execute, query, sync, Response, ResponsePayload, State};

#[derive(Debug)]
pub struct GoogleHome {
    user_id: String,
    // Add credentials so we can notify google home of actions
}

#[derive(Debug, Error)]
pub enum FullfillmentError {
    #[error("Expected at least one ResponsePayload")]
    ExpectedOnePayload,
}

impl GoogleHome {
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.into(),
        }
    }

    pub async fn handle_request<T: AsGoogleHomeDevice + ?Sized + 'static>(
        &self,
        request: Request,
        devices: &HashMap<String, Arc<RwLock<Box<T>>>>,
    ) -> Result<Response, FullfillmentError> {
        // TODO: What do we do if we actually get more then one thing in the input array, right now
        // we only respond to the first thing
        let intent = request.inputs.into_iter().next();

        let payload: OptionFuture<_> = intent
            .map(|intent| async move {
                match intent {
                    Intent::Sync => ResponsePayload::Sync(self.sync(devices).await),
                    Intent::Query(payload) => {
                        ResponsePayload::Query(self.query(payload, devices).await)
                    }
                    Intent::Execute(payload) => {
                        ResponsePayload::Execute(self.execute(payload, devices).await)
                    }
                }
            })
            .into();

        payload
            .await
            .ok_or(FullfillmentError::ExpectedOnePayload)
            .map(|payload| Response::new(&request.request_id, payload))
    }

    async fn sync<T: AsGoogleHomeDevice + ?Sized + 'static>(
        &self,
        devices: &HashMap<String, Arc<RwLock<Box<T>>>>,
    ) -> sync::Payload {
        let mut resp_payload = sync::Payload::new(&self.user_id);
        let f = devices.iter().map(|(_, device)| async move {
            if let Some(device) = device.read().await.as_ref().cast() {
                Some(device.sync().await)
            } else {
                None
            }
        });

        resp_payload.devices = join_all(f).await.into_iter().flatten().collect();
        resp_payload
    }

    async fn query<T: AsGoogleHomeDevice + ?Sized + 'static>(
        &self,
        payload: request::query::Payload,
        devices: &HashMap<String, Arc<RwLock<Box<T>>>>,
    ) -> query::Payload {
        let mut resp_payload = query::Payload::new();
        let f = payload
            .devices
            .into_iter()
            .map(|device| device.id)
            .map(|id| async move {
                // NOTE: Requires let_chains feature
                let device = if let Some(device) = devices.get(id.as_str())
                    && let Some(device) = device.read().await.as_ref().cast()
                {
                    device.query().await
                } else {
                    let mut device = query::Device::new();
                    device.set_offline();
                    device.set_error(DeviceError::DeviceNotFound.into());

                    device
                };

                (id, device)
            });

        // Await all the futures and then convert the resulting vector into a hashmap
        resp_payload.devices = join_all(f).await.into_iter().collect();
        resp_payload
    }

    async fn execute<T: AsGoogleHomeDevice + ?Sized + 'static>(
        &self,
        payload: request::execute::Payload,
        devices: &HashMap<String, Arc<RwLock<Box<T>>>>,
    ) -> execute::Payload {
        let resp_payload = Arc::new(Mutex::new(response::execute::Payload::new()));

        let f = payload.commands.into_iter().map(|command| {
            let resp_payload = resp_payload.clone();
            async move {
                let mut success = response::execute::Command::new(execute::Status::Success);
                success.states = Some(execute::States {
                    online: true,
                    state: State::default(),
                });
                let mut offline = response::execute::Command::new(execute::Status::Offline);
                offline.states = Some(execute::States {
                    online: false,
                    state: State::default(),
                });
                let mut errors: HashMap<ErrorCode, response::execute::Command> = HashMap::new();

                let f = command
                    .devices
                    .into_iter()
                    .map(|device| device.id)
                    .map(|id| {
                        let execution = command.execution.clone();
                        async move {
                            if let Some(device) = devices.get(id.as_str())
                                && let Some(device) = device.write().await.as_mut().cast_mut()
                            {
                                if !device.is_online() {
                                    return (id, Ok(false));
                                }

                                // NOTE: We can not use .map here because async =(
                                let mut results = Vec::new();
                                for cmd in &execution {
                                    results.push(device.execute(cmd).await);
                                }

                                // Convert vec of results to a result with a vec and the first
                                // encountered error
                                let results =
                                    results.into_iter().collect::<Result<Vec<_>, ErrorCode>>();

                                // TODO: We only get one error not all errors
                                if let Err(err) = results {
                                    (id, Err(err))
                                } else {
                                    (id, Ok(true))
                                }
                            } else {
                                (id.clone(), Err(DeviceError::DeviceNotFound.into()))
                            }
                        }
                    });

                let a = join_all(f).await;
                a.into_iter().for_each(|(id, state)| {
                    match state {
                        Ok(true) => success.add_id(&id),
                        Ok(false) => offline.add_id(&id),
                        Err(err) => errors
                            .entry(err)
                            .or_insert_with(|| match &err {
                                ErrorCode::DeviceError(_) => {
                                    response::execute::Command::new(execute::Status::Error)
                                }
                                ErrorCode::DeviceException(_) => {
                                    response::execute::Command::new(execute::Status::Exceptions)
                                }
                            })
                            .add_id(&id),
                    };
                });

                let mut resp_payload = resp_payload.lock().await;
                resp_payload.add_command(success);
                resp_payload.add_command(offline);
                for (error, mut cmd) in errors {
                    cmd.error_code = Some(error);
                    resp_payload.add_command(cmd);
                }
            }
        });

        join_all(f).await;

        std::sync::Arc::<tokio::sync::Mutex<response::execute::Payload>>::try_unwrap(resp_payload)
            .expect("All futures are done, so there should only be one strong reference")
            .into_inner()
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::{
//         device::{self, GoogleHomeDevice},
//         errors::ErrorCode,
//         request::Request,
//         traits, types,
//     };
//
//     #[derive(Debug)]
//     struct TestOutlet {
//         name: String,
//         on: bool,
//     }
//
//     impl TestOutlet {
//         fn new(name: &str) -> Self {
//             Self {
//                 name: name.into(),
//                 on: false,
//             }
//         }
//     }
//
//     impl GoogleHomeDevice for TestOutlet {
//         fn get_device_type(&self) -> types::Type {
//             types::Type::Outlet
//         }
//
//         fn get_device_name(&self) -> device::Name {
//             let mut name = device::Name::new("Nightstand");
//             name.add_default_name("Outlet");
//             name.add_nickname("Nightlight");
//
//             name
//         }
//
//         fn get_id(&self) -> &str {
//             &self.name
//         }
//
//         fn is_online(&self) -> bool {
//             true
//         }
//
//         fn get_room_hint(&self) -> Option<&str> {
//             Some("Bedroom")
//         }
//
//         fn get_device_info(&self) -> Option<device::Info> {
//             Some(device::Info {
//                 manufacturer: Some("Company".into()),
//                 model: Some("Outlet II".into()),
//                 hw_version: None,
//                 sw_version: None,
//             })
//         }
//     }
//
//     impl traits::OnOff for TestOutlet {
//         fn is_on(&self) -> Result<bool, ErrorCode> {
//             Ok(self.on)
//         }
//
//         fn set_on(&mut self, on: bool) -> Result<(), ErrorCode> {
//             self.on = on;
//             Ok(())
//         }
//     }
//
//     #[derive(Debug)]
//     struct TestScene;
//
//     impl TestScene {
//         fn new() -> Self {
//             Self {}
//         }
//     }
//
//     impl GoogleHomeDevice for TestScene {
//         fn get_device_type(&self) -> types::Type {
//             types::Type::Scene
//         }
//
//         fn get_device_name(&self) -> device::Name {
//             device::Name::new("Party")
//         }
//
//         fn get_id(&self) -> &str {
//             "living/party_mode"
//         }
//
//         fn is_online(&self) -> bool {
//             true
//         }
//
//         fn get_room_hint(&self) -> Option<&str> {
//             Some("Living room")
//         }
//     }
//
//     impl traits::Scene for TestScene {
//         fn set_active(&self, _activate: bool) -> Result<(), ErrorCode> {
//             println!("Activating the party scene");
//             Ok(())
//         }
//     }
//
//     #[test]
//     fn handle_sync() {
//         let json = r#"{
//   "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
//   "inputs": [
//     {
//       "intent": "action.devices.SYNC"
//     }
//   ]
// }"#;
//         let req: Request = serde_json::from_str(json).unwrap();
//
//         let gh = GoogleHome {
//             user_id: "Dreaded_X".into(),
//         };
//
//         let mut nightstand = TestOutlet::new("bedroom/nightstand");
//         let mut lamp = TestOutlet::new("living/lamp");
//         let mut scene = TestScene::new();
//         let mut devices: HashMap<&str, &mut dyn GoogleHomeDevice> = HashMap::new();
//         let id = nightstand.get_id().into();
//         devices.insert(&id, &mut nightstand);
//         let id = lamp.get_id().into();
//         devices.insert(&id, &mut lamp);
//         let id = scene.get_id().into();
//         devices.insert(&id, &mut scene);
//
//         let resp = gh.handle_request(req, &mut devices).unwrap();
//
//         let json = serde_json::to_string(&resp).unwrap();
//         println!("{}", json)
//     }
//
//     #[test]
//     fn handle_query() {
//         let json = r#"{
//   "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
//   "inputs": [
//     {
//       "intent": "action.devices.QUERY",
//       "payload": {
//         "devices": [
//           {
//             "id": "bedroom/nightstand"
//           },
//           {
//             "id": "living/party_mode"
//           }
//         ]
//       }
//     }
//   ]
// }"#;
//         let req: Request = serde_json::from_str(json).unwrap();
//
//         let gh = GoogleHome {
//             user_id: "Dreaded_X".into(),
//         };
//
//         let mut nightstand = TestOutlet::new("bedroom/nightstand");
//         let mut lamp = TestOutlet::new("living/lamp");
//         let mut scene = TestScene::new();
//         let mut devices: HashMap<&str, &mut dyn GoogleHomeDevice> = HashMap::new();
//         let id = nightstand.get_id().into();
//         devices.insert(&id, &mut nightstand);
//         let id = lamp.get_id().into();
//         devices.insert(&id, &mut lamp);
//         let id = scene.get_id().into();
//         devices.insert(&id, &mut scene);
//
//         let resp = gh.handle_request(req, &mut devices).unwrap();
//
//         let json = serde_json::to_string(&resp).unwrap();
//         println!("{}", json)
//     }
//
//     #[test]
//     fn handle_execute() {
//         let json = r#"{
//   "requestId": "ff36a3cc-ec34-11e6-b1a0-64510650abcf",
//   "inputs": [
//     {
//       "intent": "action.devices.EXECUTE",
//       "payload": {
//         "commands": [
//           {
//             "devices": [
//               {
//                 "id": "bedroom/nightstand"
//               },
//               {
//                 "id": "living/lamp"
//               }
//             ],
//             "execution": [
//               {
//                 "command": "action.devices.commands.OnOff",
//                 "params": {
//                   "on": true
//                 }
//               }
//             ]
//           }
//         ]
//       }
//     }
//   ]
// }"#;
//         let req: Request = serde_json::from_str(json).unwrap();
//
//         let gh = GoogleHome {
//             user_id: "Dreaded_X".into(),
//         };
//
//         let mut nightstand = TestOutlet::new("bedroom/nightstand");
//         let mut lamp = TestOutlet::new("living/lamp");
//         let mut scene = TestScene::new();
//         let mut devices: HashMap<&str, &mut dyn GoogleHomeDevice> = HashMap::new();
//         let id = nightstand.get_id().into();
//         devices.insert(&id, &mut nightstand);
//         let id = lamp.get_id().into();
//         devices.insert(&id, &mut lamp);
//         let id = scene.get_id().into();
//         devices.insert(&id, &mut scene);
//
//         let resp = gh.handle_request(req, &mut devices).unwrap();
//
//         let json = serde_json::to_string(&resp).unwrap();
//         println!("{}", json)
//     }
// }
