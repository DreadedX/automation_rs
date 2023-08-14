mod audio_setup;
mod contact_sensor;
mod debug_bridge;
mod hue_bridge;
mod ikea_outlet;
mod kasa_outlet;
mod light_sensor;
mod ntfy;
mod presence;
mod wake_on_lan;
mod washer;

pub use self::audio_setup::AudioSetup;
pub use self::contact_sensor::ContactSensor;
pub use self::debug_bridge::{DebugBridge, DebugBridgeConfig};
pub use self::hue_bridge::{HueBridge, HueBridgeConfig};
pub use self::ikea_outlet::IkeaOutlet;
pub use self::kasa_outlet::KasaOutlet;
pub use self::light_sensor::{LightSensor, LightSensorConfig};
pub use self::ntfy::{Notification, Ntfy};
pub use self::presence::{Presence, PresenceConfig, DEFAULT_PRESENCE};
pub use self::wake_on_lan::WakeOnLAN;
pub use self::washer::Washer;

use google_home::{device::AsGoogleHomeDevice, traits::OnOff};

use crate::traits::Timeout;
use crate::{event::OnDarkness, event::OnMqtt, event::OnNotification, event::OnPresence};

#[impl_cast::device(As: OnMqtt + OnPresence + OnDarkness + OnNotification + OnOff + Timeout)]
pub trait Device: AsGoogleHomeDevice + std::fmt::Debug + Sync + Send {
    fn get_id(&self) -> &str;
}
