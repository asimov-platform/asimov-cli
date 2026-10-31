// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, SysexitsError::*, shared::outln};
use asimov_module::ModuleName;
use color_print::{ceprintln, cformat};

pub async fn inspect(
    module_name: ModuleName,
    output: String,
    _flags: &StandardOptions,
) -> Result<(), BoxError> {
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
    super::config::validate_variables(manifest)?;

    let conf_vars = manifest
        .config
        .as_ref()
        .map(|c| c.variables.as_slice())
        .unwrap_or_default();

    let conf_status: Vec<bool> = conf_vars
        .iter()
        .map(|var| match manifest.variable(&var.name, Some("default")) {
            Ok(_) => Ok(true),
            Err(asimov_module::ReadVarError::UnconfiguredVar(_)) => Ok(false),
            Err(e) => {
                ceprintln!(
                    "<s,r>error:</> failed to read configuration variable `{}`: {e}",
                    var.name
                );
                Err(BoxError::from(EX_IOERR))
            },
        })
        .collect::<Result<_, _>>()?;

    match output.as_str() {
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

            let mut redacted = installed.clone();
            if let Some(config) = &mut redacted.manifest.config {
                for var in &mut config.variables {
                    if var.secret {
                        var.default_value = None;
                    }
                }
            }

            outln!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "manifest": serde_json::to_value(&redacted)?,
                    "enabled": is_enabled,
                    "config": config,
                }))?
            )?;
        },
        _ => {
            if is_enabled {
                outln!(
                    "{}",
                    cformat!("<s,g>✓</> <s>{}</> (enabled)", manifest.name)
                )?;
            } else {
                outln!(
                    "{}",
                    cformat!("<s,r>✗</> <s>{}</> (disabled)", manifest.name)
                )?;
            }

            if let Some(label) = &manifest.label {
                outln!("{}", cformat!("<s>Label:</> {label}"))?;
            }
            if let Some(title) = &manifest.title {
                outln!("{}", cformat!("<s>Title:</> {title}"))?;
            }
            if let Some(summary) = &manifest.summary {
                outln!("{}", cformat!("<s>Summary:</> {summary}"))?;
            }
            if let Some(version) = &installed.version {
                outln!("{}", cformat!("<s>Version:</> {version}"))?;
            }

            if !manifest.links.is_empty() {
                let mut links = manifest.links.clone();
                crate::sort_links(&manifest.name, &mut links);
                outln!("{}", cformat!("<s>Links:</>"))?;
                for link in links {
                    outln!("  {link}")?;
                }
            }

            if !manifest.provides.is_empty() {
                outln!("{}", cformat!("<s>Programs:</>"))?;
                for program in &manifest.provides.programs {
                    outln!("  {program}")?;
                }
            }

            if !manifest.handles.is_empty() {
                outln!("{}", cformat!("<s>Handles:</>"))?;
                for (kind, values) in [
                    ("URL protocols", &manifest.handles.url_protocols),
                    ("URL prefixes", &manifest.handles.url_prefixes),
                    ("URL patterns", &manifest.handles.url_patterns),
                    ("file extensions", &manifest.handles.file_extensions),
                    ("content types", &manifest.handles.content_types),
                ] {
                    if !values.is_empty() {
                        outln!("  {kind}: {}", values.join(", "))?;
                    }
                }
            }

            outln!("{}", cformat!("<s>Configuration:</>"))?;
            if conf_vars.is_empty() {
                outln!("  no configuration variables declared")?;
            } else {
                for (var, is_set) in conf_vars.iter().zip(&conf_status) {
                    if *is_set {
                        outln!("{}", cformat!("  <s,g>✓</> <s>{}</> (set)", var.name))?;
                    } else if var.is_required() {
                        outln!("{}", cformat!("  <s,r>✗</> <s>{}</> (required)", var.name))?;
                    } else {
                        outln!("{}", cformat!("  <dim>-</> <s>{}</> (unset)", var.name))?;
                    }
                    if let Some(description) = &var.description {
                        outln!("      {description}")?;
                    }
                    if let Some(default_value) = &var.default_value {
                        if var.secret {
                            outln!("      default: ******")?;
                        } else {
                            outln!("      default: {default_value}")?;
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
