// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
};
use asimov_installer::InstallOptions;
use asimov_module::{ModuleManifest, ReadVarError};
use color_print::{ceprintln, cprintln};

pub async fn install(
    module_names: &Vec<String>,
    version: &Option<String>,
    model_size: &Option<String>,
    flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let registry = asimov_registry::Registry::default();
    let installer = asimov_installer::Installer::default();

    let install_options = InstallOptions::builder()
        .maybe_version(version.clone())
        .maybe_model_size(model_size.clone())
        .build();

    let module_names = if module_names.len() == 1 && module_names[0] == "all" {
        fetch_all_module_names().await.map_err(|e| {
            tracing::error!("unable to fetch list of all modules: {e}");
            EX_UNAVAILABLE
        })?
    } else {
        module_names.clone()
    };

    for module_name in module_names {
        if !registry
            .is_module_installed(&module_name)
            .await
            .unwrap_or(false)
        {
            let target_version = if let Some(want) = version {
                want.clone()
            } else {
                installer
                    .fetch_latest_release(&module_name)
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            "unable to find latest release for module `{module_name}`: {e}"
                        );
                        EX_UNAVAILABLE
                    })?
            };

            if flags.verbose > 0 {
                cprintln!(
                    "<s,g>✓</> Found version <s>{target_version}</> for module <s>{module_name}</>."
                );
            }

            if flags.verbose > 1 {
                cprintln!("<s,c>»</> Installing module <s>{module_name}</>...");
            }

            installer
                .install_module(module_name.clone(), &install_options)
                .await
                .map_err(|e| {
                    tracing::error!("failed to install module `{module_name}`: {e}");
                    EX_UNAVAILABLE
                })?;

            if flags.verbose > 0 {
                cprintln!("<s,g>✓</> Installed module <s>{module_name}</>.");
            }
        } else if flags.verbose > 0 {
            cprintln!("<s,g>✓</> Module <s>{module_name}</> is already installed.");
        }

        if registry
            .is_module_enabled(&module_name)
            .await
            .unwrap_or(false)
        {
            continue;
        }

        let manifest = registry.read_manifest(&module_name).await.map_err(|e| {
            tracing::error!("failed to read module manifest for `{module_name}`: {e}");
            EX_UNAVAILABLE
        })?;

        let variables = manifest
            .manifest
            .config
            .iter()
            .flat_map(|conf| conf.variables.iter());

        let mut missing_variables = Vec::new();
        for var in variables {
            if var.default_value.is_some() {
                continue;
            }
            match manifest.manifest.variable(&var.name, None) {
                Ok(_) => (),
                Err(ReadVarError::UnconfiguredVar(_)) => {
                    missing_variables.push(var);
                },
                Err(e) => {
                    tracing::error!(
                        "failed to read configuration variable `{}` for module `{module_name}`: {e}",
                        var.name
                    );
                    return Err(EX_UNAVAILABLE);
                },
            }
        }

        if missing_variables.is_empty() {
            registry.enable_module(&module_name).await.map_err(|e| {
                tracing::error!("failed to enable installed module `{module_name}`: {e}");
                EX_UNAVAILABLE
            })?;
        } else {
            ceprintln!(
                "<s,y>warn:</> Module <s>{module_name}</> wasn't enabled automatically due to missing configuration."
            );
            ceprintln!("<s,dim>hint:</> Module <s>{module_name}</> requires configuration:");

            for var in missing_variables {
                let desc_suffix = if let Some(ref desc) = var.description {
                    format!(" (Description: \"{desc}\")")
                } else {
                    String::new()
                };

                ceprintln!(
                    "<s,dim>hint:</>   Missing variable: <s>{}</s>{}",
                    var.name,
                    desc_suffix
                );

                if let Some(ref env) = var.environment {
                    ceprintln!(
                        "<s,dim>hint:</>   Alternative: set environment variable: <s>{env}</>"
                    );
                }
            }

            ceprintln!("<s,dim>hint:</>   To configure: <s>asimov module config {module_name}</s>");
            ceprintln!("<s,dim>hint:</>   To enable: <s>asimov module enable {module_name}</s>");
        }
    }

    Ok(())
}

pub async fn fetch_all_module_names() -> Result<Vec<String>, Box<dyn core::error::Error>> {
    let url = "https://github.com/asimov-modules/asimov-modules/raw/master/all/.asimov/module.yaml";

    let client = reqwest::Client::builder()
        .user_agent("asimov-module-cli")
        .connect_timeout(std::time::Duration::from_secs(10))
        .read_timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    if !response.status().is_success() {
        Err(format!(
            "HTTP status code was not successful: {0}",
            response.status()
        ))?;
    }

    let content = response
        .text()
        .await
        .inspect_err(|err| tracing::debug!(?err))?;

    let manifest: ModuleManifest = serde_yml::from_str(&content)
        .inspect_err(|err| tracing::debug!(?err, ?content))
        .map_err(|e| format!("unable to deserialize GitHub response: {e}"))?;

    Ok(manifest.requires.modules)
}
