// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError::*};
use color_print::{ceprintln, cprintln};
use core::error::Error;

pub async fn inspect(
    module_name: &str,
    output: &str,
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    let module_name = module_name.parse()?;
    let registry = asimov_registry::Registry::default();

    let installed = registry.read_manifest(&module_name).await.map_err(|e| {
        tracing::error!("failed to read manifest for module `{module_name}`: {e}");
        if let asimov_registry::error::ManifestError::NotInstalled = e {
            ceprintln!(
                "<s,dim>hint:</> Check if the module is installed with: <s>asimov module list</>"
            );
        }
        EX_UNAVAILABLE
    })?;

    let is_enabled = registry
        .is_module_enabled(&module_name)
        .await
        .map_err(|e| {
            tracing::error!("failed to check if module is enabled: {e}");
            EX_UNAVAILABLE
        })?;

    let manifest = &installed.manifest;

    let conf_vars = manifest
        .config
        .as_ref()
        .map(|c| c.variables.as_slice())
        .unwrap_or_default();

    let conf_status: Vec<bool> = conf_vars
        .iter()
        .map(|var| match manifest.variable(&var.name, Some("default")) {
            Ok(_) => true,
            Err(asimov_module::ReadVarError::UnconfiguredVar(_)) => false,
            Err(e) => {
                tracing::warn!("failed to read configuration variable `{}`: {e}", var.name);
                false
            },
        })
        .collect();

    match output {
        "json" => {
            let config: Vec<serde_json::Value> = conf_vars
                .iter()
                .zip(&conf_status)
                .map(|(var, is_set)| {
                    serde_json::json!({
                        "name": var.name,
                        "description": var.description,
                        "default": var.default_value.as_deref().filter(|_| !var.secret),
                        "secret": var.secret,
                        "required": var.is_required(),
                        "set": is_set,
                    })
                })
                .collect();

            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "manifest": serde_json::to_value(&installed)?,
                    "enabled": is_enabled,
                    "config": config,
                }))?
            );
        },
        _ => {
            if is_enabled {
                cprintln!("<s,g>✓</> <s>{}</> (enabled)", manifest.name);
            } else {
                cprintln!("<s,r>✗</> <s>{}</> (disabled)", manifest.name);
            }

            if let Some(label) = &manifest.label {
                cprintln!("<s>Label:</> {label}");
            }
            if let Some(title) = &manifest.title {
                cprintln!("<s>Title:</> {title}");
            }
            if let Some(summary) = &manifest.summary {
                cprintln!("<s>Summary:</> {summary}");
            }
            if let Some(version) = &installed.version {
                cprintln!("<s>Version:</> {version}");
            }

            if !manifest.links.is_empty() {
                let mut links = manifest.links.clone();
                crate::sort_links(&manifest.name, &mut links);
                cprintln!("<s>Links:</>");
                for link in links {
                    println!("  {link}");
                }
            }

            if !manifest.provides.is_empty() {
                cprintln!("<s>Programs:</>");
                for program in &manifest.provides.programs {
                    println!("  {program}");
                }
            }

            if !manifest.handles.is_empty() {
                cprintln!("<s>Handles:</>");
                for (kind, values) in [
                    ("URL protocols", &manifest.handles.url_protocols),
                    ("URL prefixes", &manifest.handles.url_prefixes),
                    ("URL patterns", &manifest.handles.url_patterns),
                    ("file extensions", &manifest.handles.file_extensions),
                    ("content types", &manifest.handles.content_types),
                ] {
                    if !values.is_empty() {
                        println!("  {kind}: {}", values.join(", "));
                    }
                }
            }

            cprintln!("<s>Configuration:</>");
            if conf_vars.is_empty() {
                println!("  no configuration variables declared");
            } else {
                for (var, is_set) in conf_vars.iter().zip(&conf_status) {
                    if *is_set {
                        cprintln!("  <s,g>✓</> <s>{}</> (set)", var.name);
                    } else if var.is_required() {
                        cprintln!("  <s,r>✗</> <s>{}</> (required)", var.name);
                    } else {
                        cprintln!("  <dim>-</> <s>{}</> (unset)", var.name);
                    }
                    if let Some(description) = &var.description {
                        println!("      {description}");
                    }
                    if let Some(default_value) = &var.default_value {
                        if var.secret {
                            println!("      default: ******");
                        } else {
                            println!("      default: {default_value}");
                        }
                    }
                }
            }
        },
    }

    // The report is the output; whether the module is ready to use is the
    // exit status, so that inspecting one doubles as checking it.
    let missing: Vec<&str> = conf_vars
        .iter()
        .zip(&conf_status)
        .filter(|(var, is_set)| var.is_required() && !**is_set)
        .map(|(var, _)| var.name.as_str())
        .collect();

    if !missing.is_empty() {
        ceprintln!(
            "<s,dim>hint:</> Configure the missing variable(s) interactively with: <s>asimov module config setup {module_name}</>"
        );
        return Err(EX_CONFIG.into());
    }

    Ok(())
}
