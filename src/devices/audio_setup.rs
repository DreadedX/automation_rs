use std::io::{Write, Read};
use std::net::{TcpStream, SocketAddr};

use bytes::{BufMut, Buf};
use google_home::errors::{ErrorCode, DeviceError};
use google_home::traits::{self, OnOff};
use rumqttc::AsyncClient;
use serde::{Deserialize, Serialize};
use tracing::warn;
use pollster::FutureExt as _;

use crate::config::MqttDeviceConfig;
use crate::mqtt::{OnMqtt, RemoteMessage, RemoteAction};

use super::Device;

struct TPLinkOutlet {
    addr: SocketAddr,
}

impl TPLinkOutlet {
    pub fn new(ip: [u8; 4]) -> Self {
        // @TODO Get the current state of the outlet
        Self { addr: (ip, 9999).into() }
    }

    pub fn encrypt(data: bytes::Bytes) -> bytes::Bytes {
        let mut key: u8 = 171;
        let mut encrypted = bytes::BytesMut::with_capacity(data.len() + 4);

        encrypted.put_u32(data.len() as u32);

        for c in data {
            key = key ^ c;
            encrypted.put_u8(key);
        }

        return encrypted.freeze();
    }
}
#[derive(Debug, Serialize)]
struct RequestRelayState {
    state: isize,
}

#[derive(Debug, Serialize)]
struct RequestSysinfo;

#[derive(Debug, Serialize)]
struct RequestSystem {
    #[serde(skip_serializing_if = "Option::is_none")]
    get_sysinfo: Option<RequestSysinfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    set_relay_state: Option<RequestRelayState>,
}

#[derive(Debug, Serialize)]
struct Request {
    system: RequestSystem,
}

impl Request {
    fn get_sysinfo() -> Self {
        Self {
            system: RequestSystem {
                get_sysinfo: Some(RequestSysinfo{}),
                set_relay_state: None
            }
        }
    }

    fn set_relay_state(on: bool) -> Self {
        Self {
            system: RequestSystem {
                get_sysinfo: None,
                set_relay_state: Some(RequestRelayState {
                    state: if on { 1 } else { 0 }
                })
            }
        }
    }


    fn encrypt(&self) -> bytes::Bytes {
        let data: bytes::Bytes = serde_json::to_string(self).unwrap().into();

        let mut key: u8 = 171;
        let mut encrypted = bytes::BytesMut::with_capacity(data.len() + 4);

        encrypted.put_u32(data.len() as u32);

        for c in data {
            key = key ^ c;
            encrypted.put_u8(key);
        }

        return encrypted.freeze();
    }
}

#[derive(Debug, Deserialize)]
struct ResponseSetRelayState {
    err_code: isize,
}

#[derive(Debug, Deserialize)]
struct ResponseGetSysinfo {
    err_code: isize,
    relay_state: isize,
}

#[derive(Debug, Deserialize)]
struct ResponseSystem {
    set_relay_state: Option<ResponseSetRelayState>,
    get_sysinfo: Option<ResponseGetSysinfo>,
}

#[derive(Debug, Deserialize)]
struct Response {
    system: ResponseSystem,
}

impl Response {
    fn get_current_relay_state(&self) -> Result<bool, anyhow::Error> {
        if let Some(sysinfo) = &self.system.get_sysinfo {
            if sysinfo.err_code != 0 {
                return Err(anyhow::anyhow!("Error code: {}", sysinfo.err_code));
            }
            return Ok(sysinfo.relay_state == 1);
        }

        return Err(anyhow::anyhow!("No sysinfo found in response"));
    }

    fn check_set_relay_success(&self) -> Result<(), anyhow::Error> {
        if let Some(set_relay_state) = &self.system.set_relay_state {
            if set_relay_state.err_code != 0 {
                return Err(anyhow::anyhow!("Error code: {}", set_relay_state.err_code));
            }
            return Ok(());
        }

        return Err(anyhow::anyhow!("No relay_state found in response"));
    }

    fn decrypt(mut data: bytes::Bytes) -> Result<Self, anyhow::Error> {
        let mut key: u8 = 171;
        if data.len() < 4 {
            return Err(anyhow::anyhow!("Expected a minimun data length of 4"));
        }

        let length = data.get_u32();
        let mut decrypted = bytes::BytesMut::with_capacity(length as usize);

        for c in data {
            decrypted.put_u8(key ^ c);
            key = c;
        }

        let decrypted = std::str::from_utf8(&decrypted)?;
        Ok(serde_json::from_str(decrypted)?)
    }
}

impl traits::OnOff for TPLinkOutlet {
    fn is_on(&self) -> Result<bool, ErrorCode> {
        let mut stream = TcpStream::connect(self.addr).or::<DeviceError>(Err(DeviceError::DeviceOffline.into()))?;

        let body = Request::get_sysinfo().encrypt();
        stream.write_all(&body).and(stream.flush()).or::<DeviceError>(Err(DeviceError::TransientError.into()))?;

        let mut received = Vec::new();
        let mut rx_bytes = [0; 1024];
        loop {
            let read = stream.read(&mut rx_bytes).or::<ErrorCode>(Err(DeviceError::TransientError.into()))?;

            received.extend_from_slice(&rx_bytes[..read]);

            if read < rx_bytes.len() {
                break;
            }
        }

        let resp = Response::decrypt(received.into()).or::<ErrorCode>(Err(DeviceError::TransientError.into()))?;

        resp.get_current_relay_state().or(Err(DeviceError::TransientError.into()))
    }

    fn set_on(&mut self, on: bool) -> Result<(), ErrorCode> {
        let mut stream = TcpStream::connect(self.addr).or::<DeviceError>(Err(DeviceError::DeviceOffline.into()))?;

        let body = Request::set_relay_state(on).encrypt();
        stream.write_all(&body).and(stream.flush()).or::<DeviceError>(Err(DeviceError::TransientError.into()))?;

        let mut received = Vec::new();
        let mut rx_bytes = [0; 1024];
        loop {
            let read = match stream.read(&mut rx_bytes) {
                Ok(read) => read,
                Err(_) => return Err(DeviceError::TransientError.into()),
            };

            received.extend_from_slice(&rx_bytes[..read]);

            if read < rx_bytes.len() {
                break;
            }
        }

        let resp = Response::decrypt(received.into()).or::<ErrorCode>(Err(DeviceError::TransientError.into()))?;

        resp.check_set_relay_success().or(Err(DeviceError::TransientError.into()))
    }
}

pub struct AudioSetup {
    identifier: String,
    mqtt: MqttDeviceConfig,
    mixer: TPLinkOutlet,
    speakers: TPLinkOutlet,
}

impl AudioSetup {
    pub fn new(identifier: String, mqtt: MqttDeviceConfig, mixer_ip: [u8; 4], speakers_ip: [u8; 4], client: AsyncClient) -> Self {
        let mixer = TPLinkOutlet::new(mixer_ip);
        let speakers = TPLinkOutlet::new(speakers_ip);

        client.subscribe(mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce).block_on().unwrap();

        Self { identifier, mqtt, mixer, speakers }
    }
}

impl Device for AudioSetup {
    fn get_id(&self) -> String {
        self.identifier.clone()
    }
}

impl OnMqtt for AudioSetup {
    fn on_mqtt(&mut self, message: &rumqttc::Publish) {
        if message.topic != self.mqtt.topic {
            return;
        }

        let action = match RemoteMessage::try_from(message) {
            Ok(message) => message.action(),
            Err(err) => {
                warn!(id = self.identifier, "Failed to parse message: {err}");
                return;
            }
        };

        match action {
            RemoteAction::On => {
                if self.mixer.is_on().unwrap() {
                    self.speakers.set_on(false).unwrap();
                    self.mixer.set_on(false).unwrap();
                } else {
                    self.speakers.set_on(true).unwrap();
                    self.mixer.set_on(true).unwrap();
                }
            },
            RemoteAction::BrightnessMoveUp => {
                if !self.mixer.is_on().unwrap() {
                    self.mixer.set_on(true).unwrap();
                } else if self.speakers.is_on().unwrap() {
                    self.speakers.set_on(false).unwrap();
                } else {
                    self.speakers.set_on(true).unwrap();
                }
            },
            RemoteAction::BrightnessStop => { /* Ignore this action */ },
            _ => warn!("Expected ikea shortcut button which only supports 'on' and 'brightness_move_up', got: {action:?}")
        }
    }
}
