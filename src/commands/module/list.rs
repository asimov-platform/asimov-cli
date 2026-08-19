// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, SysexitsError::*};
use color_print::cprintln;

pub async fn list(output: String, _flags: &StandardOptions) -> Result<(), BoxError> {
    let registry = asimov_registry::Registry::default();
    let modules = registry.installed_modules().await.map_err(|e| {
        tracing::error!("failed to read installed modules: {e}");
        EX_UNAVAILABLE
    })?;

    for module in modules {
        let name = module.manifest.name.parse()?;
        let is_enabled = registry.is_module_enabled(&name).await.map_err(|e| {
            tracing::error!("failed to check if module is enabled: {e}");
            EX_UNAVAILABLE
        })?;

        match output.as_str() {
            "jsonl" => {
                let version = module.version.unwrap_or_default();
                let label = module.manifest.label;
                let uri = format!("https://asimov.directory/modules/{name}");
                println!(
                    r#"{{"@type": "AsimovModule", "@id": "{}", "name": "{}", "label": "{}", "enabled": {}, "version": "{}"}}"#,
                    uri,
                    name,
                    label.unwrap_or_default(),
                    is_enabled,
                    version
                );
            },
            _ => {
                if is_enabled {
                    cprintln!("<s,g>✓</> {}", name);
                } else {
                    cprintln!("<s,r>✗</> {}", name);
                }
            },
        }
    }

    Ok(())
}
