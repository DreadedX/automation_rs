use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    str::Utf8Error,
};

use bytes::{Buf, BufMut};
use google_home::{
    errors::{self, DeviceError},
    traits,
};
use rumqttc::AsyncClient;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::trace;

use crate::{config::CreateDevice, error::CreateDeviceError};

use super::Device;

#[derive(Debug, Clone, Deserialize)]
pub struct KasaOutletConfig {
    ip: Ipv4Addr,
}

#[derive(Debug)]
pub struct KasaOutlet {
    identifier: String,
    addr: SocketAddr,
}

impl CreateDevice for KasaOutlet {
    type Config = KasaOutletConfig;

    fn create(
        identifier: &str,
        config: Self::Config,
        _client: AsyncClient,
        _presence_topic: &str,
    ) -> Result<Self, CreateDeviceError> {
        trace!(id = identifier, "Setting up KasaOutlet");

        Ok(Self {
            identifier: identifier.to_owned(),
            addr: (config.ip, 9999).into(),
        })
    }
}

impl Device for KasaOutlet {
    fn get_id(&self) -> &str {
        &self.identifier
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
                get_sysinfo: Some(RequestSysinfo {}),
                set_relay_state: None,
            },
        }
    }

    fn set_relay_state(on: bool) -> Self {
        Self {
            system: RequestSystem {
                get_sysinfo: None,
                set_relay_state: Some(RequestRelayState {
                    state: if on { 1 } else { 0 },
                }),
            },
        }
    }

    fn encrypt(&self) -> bytes::Bytes {
        let data: bytes::Bytes = serde_json::to_string(self).unwrap().into();

        let mut key: u8 = 171;
        let mut encrypted = bytes::BytesMut::with_capacity(data.len() + 4);

        encrypted.put_u32(data.len() as u32);

        for c in data {
            key ^= c;
            encrypted.put_u8(key);
        }

        encrypted.freeze()
    }
}

#[derive(Debug, Deserialize)]
struct ErrorCode {
    err_code: isize,
}

impl ErrorCode {
    fn ok(&self) -> Result<(), ResponseError> {
        if self.err_code != 0 {
            Err(ResponseError::ErrorCode(self.err_code))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Deserialize)]
struct ResponseSetRelayState {
    #[serde(flatten)]
    err_code: ErrorCode,
}

#[derive(Debug, Deserialize)]
struct ResponseGetSysinfo {
    #[serde(flatten)]
    err_code: ErrorCode,
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

// TODO: Improve this error
#[derive(Debug, Error)]
enum ResponseError {
    #[error("Expected a minimum data length of 4")]
    ToShort,
    #[error("No sysinfo found in response")]
    SysinfoNotFound,
    #[error("No relay_state not found in response")]
    RelayStateNotFound,
    #[error("Error code: {0}")]
    ErrorCode(isize),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error>),
}

impl From<Utf8Error> for ResponseError {
    fn from(err: Utf8Error) -> Self {
        ResponseError::Other(err.into())
    }
}

impl From<serde_json::Error> for ResponseError {
    fn from(err: serde_json::Error) -> Self {
        ResponseError::Other(err.into())
    }
}

impl Response {
    fn get_current_relay_state(&self) -> Result<bool, ResponseError> {
        if let Some(sysinfo) = &self.system.get_sysinfo {
            return sysinfo.err_code.ok().map(|_| sysinfo.relay_state == 1);
        }

        Err(ResponseError::SysinfoNotFound)
    }

    fn check_set_relay_success(&self) -> Result<(), ResponseError> {
        if let Some(set_relay_state) = &self.system.set_relay_state {
            return set_relay_state.err_code.ok();
        }

        Err(ResponseError::RelayStateNotFound)
    }

    fn decrypt(mut data: bytes::Bytes) -> Result<Self, ResponseError> {
        let mut key: u8 = 171;
        if data.len() < 4 {
            return Err(ResponseError::ToShort);
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

impl traits::OnOff for KasaOutlet {
    fn is_on(&self) -> Result<bool, errors::ErrorCode> {
        let mut stream =
            TcpStream::connect(self.addr).or::<DeviceError>(Err(DeviceError::DeviceOffline))?;

        let body = Request::get_sysinfo().encrypt();
        stream
            .write_all(&body)
            .and(stream.flush())
            .or::<DeviceError>(Err(DeviceError::TransientError))?;

        let mut received = Vec::new();
        let mut rx_bytes = [0; 1024];
        loop {
            let read = stream
                .read(&mut rx_bytes)
                .or::<errors::ErrorCode>(Err(DeviceError::TransientError.into()))?;

            received.extend_from_slice(&rx_bytes[..read]);

            if read < rx_bytes.len() {
                break;
            }
        }

        let resp = Response::decrypt(received.into())
            .or::<errors::ErrorCode>(Err(DeviceError::TransientError.into()))?;

        resp.get_current_relay_state()
            .or(Err(DeviceError::TransientError.into()))
    }

    fn set_on(&mut self, on: bool) -> Result<(), errors::ErrorCode> {
        let mut stream =
            TcpStream::connect(self.addr).or::<DeviceError>(Err(DeviceError::DeviceOffline))?;

        let body = Request::set_relay_state(on).encrypt();
        stream
            .write_all(&body)
            .and(stream.flush())
            .or::<DeviceError>(Err(DeviceError::TransientError))?;

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

        let resp = Response::decrypt(received.into())
            .or::<errors::ErrorCode>(Err(DeviceError::TransientError.into()))?;

        resp.check_set_relay_success()
            .or(Err(DeviceError::TransientError.into()))
    }
}
