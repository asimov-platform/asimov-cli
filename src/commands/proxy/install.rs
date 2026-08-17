// This is free and unencumbered software released into the public domain.

use super::{ProxyInstallTarget, zed_asimov_provider};
use crate::StandardOptions;
use core::error::Error;
use jsonc_parser::{ParseOptions, cst::CstRootNode};
use std::path::PathBuf;

pub async fn install(
    apps: &[ProxyInstallTarget],
    flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    use ProxyInstallTarget::*;
    let home_path = dirs::home_dir().expect("HOME should be set");
    let apps = if apps.is_empty() {
        &[
            #[cfg(feature = "unstable")]
            Cursor,
            #[cfg(feature = "unstable")]
            Obsidian,
            #[cfg(feature = "unstable")]
            VSCode,
            Zed,
        ]
    } else {
        apps
    };
    for app in apps {
        install_app(*app, &home_path, flags).await?;
    }
    Ok(())
}

pub async fn install_app(
    app: ProxyInstallTarget,
    home_path: &PathBuf,
    flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    use ProxyInstallTarget::*;
    match app {
        #[cfg(feature = "unstable")]
        Cursor => {
            // See: https://www.jackyoustra.com/blog/cursor-settings-location
            todo!() // TODO
        },

        #[cfg(feature = "unstable")]
        Obsidian => {
            todo!() // TODO
        },

        #[cfg(feature = "unstable")]
        VSCode => {
            todo!() // TODO
        },

        Zed => {
            // See: https://zed.dev/docs/reference/all-settings#language-models
            if flags.verbose > 0 {
                eprintln!("Configuring Zed...");
            }
            let zed_path = home_path.join(".config/zed/settings.json");
            if !zed_path.exists() {
                eprintln!("error: {} not found.", zed_path.display());
                return Ok(());
            }
            let zed_input = std::fs::read_to_string(&zed_path).unwrap_or_else(|_| "{}".to_string());
            let cst = CstRootNode::parse(&zed_input, &ParseOptions::default())?;
            let root = cst.object_value_or_set();
            if let Some(models) = root.get("language_models").and_then(|p| p.object_value())
                && let Some(openai) = models
                    .get("openai_compatible")
                    .and_then(|p| p.object_value())
            {
                if let Some(asimov) = openai.get("ASIMOV") {
                    asimov.set_value(zed_asimov_provider());
                } else {
                    openai.append("ASIMOV", zed_asimov_provider());
                }
            }
            let zed_output = cst.to_string();
            //eprintln!("{}", &zed_output); // DEBUG
            std::fs::write(&zed_path, zed_output)?;
            if flags.verbose > 0 {
                eprintln!("Configured Zed: {}", zed_path.display());
            }
        },
    };
    Ok(())
}
