use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum Type {
    #[serde(rename = "action.devices.types.KETTLE")]
    Kettle,
    #[serde(rename = "action.devices.types.OUTLET")]
    Outlet,
    #[serde(rename = "action.devices.types.GRILL")]
    Grill,
    #[serde(rename = "action.devices.types.SCENE")]
    Scene,
}
