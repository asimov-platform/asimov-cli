// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use core::error::Error;
use std::fs::exists;
use tracing::{info, warn};

pub async fn init(name: &Option<String>, _flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
    // mkdir -p .asimov/
    if !exists(".asimov")? {
        info!("Creating the directory `{}`...", ".asimov");
        std::fs::create_dir_all(".asimov/")?;
        warn!("Created the directory `{}`.", ".asimov");
    }

    // echo "name: $NAME" > .asimov/module.yaml
    if !exists(".asimov/module.yaml")? {
        let module_name = if let Some(name) = name {
            name.clone()
        } else if exists("Cargo.toml")? {
            let cargo_toml = distrib::rust::load_cargo_toml("Cargo.toml")?;
            let package_name = cargo_toml.package().name();
            package_name
                .strip_prefix("asimov-")
                .and_then(|s| s.strip_suffix("-module"))
                .unwrap_or(package_name)
                .to_string()
        } else {
            "mymodule".to_string() // a dummy default name
        };
        info!("Creating the file `{}`...", ".asimov/module.yaml");
        std::fs::write(
            ".asimov/module.yaml",
            format!(
                "# See: https://asimov-specs.github.io/module-manifest/\n---\nname: {}\n",
                module_name
            ),
        )?;
        warn!("Created the file `{}`.", ".asimov/module.yaml");
    }

    Ok(())
}
