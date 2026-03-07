// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
};
use asimov_installer::InstallOptions;
use color_print::cprintln;

pub async fn upgrade(
    module_names: &Vec<String>,
    version: &Option<String>,
    model_size: &Option<String>,
    flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let registry = asimov_registry::Registry::default();
    let installer = asimov_installer::Installer::default();

    let module_names = if !module_names.is_empty() {
        module_names.clone()
    } else {
        registry
            .installed_modules()
            .await
            .map_err(|e| {
                tracing::error!("failed to read installed modules: {e}");
                EX_UNAVAILABLE
            })?
            .into_iter()
            .map(|manifest| manifest.manifest.name)
            .collect()
    };

    let install_options = InstallOptions::builder()
        .maybe_version(version.clone())
        .maybe_model_size(model_size.clone())
        .build();

    for module_name in module_names {
        let current = registry.module_version(&module_name).await.map_err(|_| {
            tracing::error!("failed to read installed version of `{module_name}`");
            EX_UNAVAILABLE
        })?;

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

        if current.is_some_and(|current| current == target_version) {
            if flags.verbose > 0 {
                let vers_txt = if version.is_some() {
                    "version"
                } else {
                    "latest version"
                };

                cprintln!(
                    "<s,g>✓</> Module <s>{module_name}</> already has {vers_txt} <s>{target_version}</> installed."
                );
            }
            continue;
        }

        if flags.verbose > 1 {
            cprintln!("<s,c>»</> Upgrading module <s>{module_name}</>...");
        }

        installer
            .upgrade_module(module_name.clone(), &install_options)
            .await
            .map_err(|e| {
                tracing::error!("module upgrade failed for `{module_name}`: {e}");
                EX_UNAVAILABLE
            })?;

        if flags.verbose > 0 {
            cprintln!(
                "<s,g>✓</> Upgraded module <s>{module_name}</> to version <s>{target_version}</>."
            );
        }
    }
    Ok(())
}
