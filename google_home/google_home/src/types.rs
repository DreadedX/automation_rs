use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum Type {
    #[serde(rename = "action.devices.types.KETTLE")]
    Kettle,
    #[serde(rename = "action.devices.types.OUTLET")]
    Outlet,
    #[serde(rename = "action.devices.types.LIGHT")]
    Light,
    #[serde(rename = "action.devices.types.SCENE")]
    Scene,
    #[serde(rename = "action.devices.types.AIRPURIFIER")]
    AirPurifier,
    #[serde(rename = "action.devices.types.DOOR")]
    Door,
    #[serde(rename = "action.devices.types.WINDOW")]
    Window,
    #[serde(rename = "action.devices.types.DRAWER")]
    Drawer,
}
