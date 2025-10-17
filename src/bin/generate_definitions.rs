use std::fs::{self, File};
use std::io::Write;

use automation::config::{Config, FulfillmentConfig};
use automation_lib::Module;
use lua_typed::Typed;
use tracing::{info, warn};

extern crate automation_devices;

fn write_definitions(filename: &str, definitions: &str) -> std::io::Result<()> {
    let definitions_directory =
        std::path::Path::new(std::env!("CARGO_MANIFEST_DIR")).join("definitions");
    fs::create_dir_all(&definitions_directory)?;

    let mut file = File::create(definitions_directory.join(filename))?;

    file.write_all(b"-- DO NOT MODIFY, FILE IS AUTOMATICALLY GENERATED\n")?;
    file.write_all(definitions.as_bytes())?;

    // Make sure we have a trailing new line
    if !definitions.ends_with("\n") {
        file.write_all(b"\n")?;
    }

    Ok(())
}

fn config_definitions() -> String {
    let mut output = "---@meta\n\n".to_string();

    output +=
        &FulfillmentConfig::generate_full().expect("FulfillmentConfig should have a definition");
    output += "\n";
    output += &Config::generate_full().expect("Config should have a definition");

    output
}

fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    for module in inventory::iter::<Module> {
        if let Some(definitions) = module.definitions() {
            info!(name = module.get_name(), "Generating definitions");

            let filename = format!("{}.lua", module.get_name());
            write_definitions(&filename, &definitions)?;
        } else {
            warn!(name = module.get_name(), "No definitions");
        }
    }

    write_definitions("config.lua", &config_definitions())?;

    Ok(())
}
