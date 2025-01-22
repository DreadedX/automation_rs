use async_trait::async_trait;
use serde::Serialize;

use crate::errors::ErrorCode;
use crate::response;
use crate::traits::{Command, DeviceFulfillment};
use crate::types::Type;

#[async_trait]
pub trait Device: DeviceFulfillment {
    fn get_device_type(&self) -> Type;
    fn get_device_name(&self) -> Name;
    fn get_id(&self) -> String;
    async fn is_online(&self) -> bool;

    // Default values that can optionally be overridden
    fn will_report_state(&self) -> bool {
        false
    }
    fn get_room_hint(&self) -> Option<&str> {
        None
    }
    fn get_device_info(&self) -> Option<Info> {
        None
    }

    async fn sync(&self) -> response::sync::Device {
        let name = self.get_device_name();
        let mut device =
            response::sync::Device::new(&self.get_id(), &name.name, self.get_device_type());

        device.name = name;
        device.will_report_state = self.will_report_state();
        // notification_supported_by_agent
        if let Some(room) = self.get_room_hint() {
            device.room_hint = Some(room.into());
        }
        device.device_info = self.get_device_info();

        // TODO: Return the appropriate error
        if let Ok((traits, attributes)) = DeviceFulfillment::sync(self).await {
            device.traits = traits;
            device.attributes = attributes;
        }

        device
    }

    async fn query(&self) -> response::query::Device {
        let mut device = response::query::Device::new();
        if !self.is_online().await {
            device.set_offline();
        }

        // TODO: Return the appropriate error
        if let Ok(state) = DeviceFulfillment::query(self).await {
            device.state = state;
        }

        device
    }

    async fn execute(&self, command: Command) -> Result<(), ErrorCode> {
        // TODO: Do something with the return value, or just get rut of the return value?
        if DeviceFulfillment::execute(self, command.clone())
            .await
            .is_err()
        {
            return Err(ErrorCode::DeviceError(
                crate::errors::DeviceError::TransientError,
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Name {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    default_names: Vec<String>,
    name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    nicknames: Vec<String>,
}

impl Name {
    pub fn new(name: &str) -> Self {
        Self {
            default_names: Vec::new(),
            name: name.into(),
            nicknames: Vec::new(),
        }
    }

    pub fn add_default_name(&mut self, name: &str) {
        self.default_names.push(name.into());
    }

    pub fn add_nickname(&mut self, name: &str) {
        self.nicknames.push(name.into());
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hw_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sw_version: Option<String>,
    // attributes
    // customData
    // otherDeviceIds
}
