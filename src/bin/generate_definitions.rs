use std::fs::{self, File};
use std::io::Write;

use automation_lib::Module;
use tracing::{info, warn};

extern crate automation_devices;

fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let definitions_directory =
        std::path::Path::new(std::env!("CARGO_MANIFEST_DIR")).join("definitions");
    fs::create_dir_all(&definitions_directory)?;

    for module in inventory::iter::<Module> {
        if let Some(definitions) = module.definitions() {
            info!(name = module.get_name(), "Generating definitions");

            let filename = format!("{}.lua", module.get_name());
            let mut file = File::create(definitions_directory.join(filename))?;

            file.write_all(b"-- DO NOT MODIFY, FILE IS AUTOMATICALLY GENERATED\n")?;
            file.write_all(definitions.as_bytes())?;
        } else {
            warn!(name = module.get_name(), "No definitions");
        }
    }

    Ok(())
    // automation_devices::generate_definitions()
}
