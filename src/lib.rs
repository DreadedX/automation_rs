#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(let_chains)]

use once_cell::sync::Lazy;
use tokio::sync::Mutex;
pub mod auth;
pub mod config;
pub mod device_manager;
pub mod devices;
pub mod error;
pub mod event;
pub mod messages;
pub mod mqtt;
pub mod schedule;
pub mod traits;

pub static LUA: Lazy<Mutex<mlua::Lua>> = Lazy::new(|| Mutex::new(mlua::Lua::new()));
