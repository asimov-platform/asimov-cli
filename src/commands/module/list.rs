// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, SysexitsError::*};
use color_print::cformat;

#[derive(serde::Serialize)]
struct ModuleRecord<'a> {
    #[serde(rename = "@type")]
    kind: &'static str,
    #[serde(rename = "@id")]
    uri: String,
    name: &'a str,
    label: &'a str,
    enabled: bool,
    version: &'a str,
}

pub async fn list(output: String, flags: &StandardOptions) -> Result<(), BoxError> {
    let color = crate::shared::stdout_color(flags);
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
                let record = ModuleRecord {
                    kind: "AsimovModule",
                    uri: format!("https://asimov.directory/modules/{name}"),
                    name: &module.manifest.name,
                    label: module.manifest.label.as_deref().unwrap_or_default(),
                    enabled: is_enabled,
                    version: module.version.as_deref().unwrap_or_default(),
                };
                println!("{}", serde_json::to_string(&record)?);
            },
            _ => {
                let line = if is_enabled {
                    cformat!("<s,g>✓</> {}", name)
                } else {
                    cformat!("<s,r>✗</> {}", name)
                };
                println!(
                    "{}",
                    if color {
                        line
                    } else {
                        clientele::strip_ansi(&line)
                    }
                );
            },
        }
    }

    Ok(())
}
