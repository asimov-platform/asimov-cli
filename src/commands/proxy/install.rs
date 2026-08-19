// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions};
use clap::ValueEnum;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ProxyInstallTarget {
    /// Cursor (https://zed.dev).
    #[cfg(feature = "unstable")]
    Cursor,

    /// Obsidian (https://obsidian.md).
    #[cfg(feature = "unstable")]
    Obsidian,

    /// Visual Studio Code (https://code.visualstudio.com).
    #[cfg(feature = "unstable")]
    VSCode,

    /// Zed (https://zed.dev).
    Zed,
}

pub async fn install(
    mut apps: Vec<ProxyInstallTarget>,
    flags: &StandardOptions,
) -> Result<(), BoxError> {
    use ProxyInstallTarget::*;
    let home_path = dirs::home_dir().expect("HOME should be set");
    if apps.is_empty() {
        apps.extend_from_slice(&[
            #[cfg(feature = "unstable")]
            Cursor,
            #[cfg(feature = "unstable")]
            Obsidian,
            #[cfg(feature = "unstable")]
            VSCode,
            Zed,
        ]);
    }
    for app in apps {
        install_app(app, &home_path, flags).await?;
    }
    Ok(())
}

pub async fn install_app(
    app: ProxyInstallTarget,
    home_path: &PathBuf,
    flags: &StandardOptions,
) -> Result<(), BoxError> {
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
            let path = home_path.join(".config/zed/settings.json");
            if !path.exists() {
                eprintln!("error: {} not found.", path.display());
                return Ok(());
            }
            patch_jsonc_file_with_edikt(
                &path,
                &["language_models", "openai_compatible", "ASIMOV"],
                include_str!("config/zed-provider.jsonc"),
            )?;
            if flags.verbose > 0 {
                eprintln!("Configured Zed: {}", path.display());
            }
        },
    };
    Ok(())
}

fn patch_jsonc_file_with_edikt(
    file_path: impl AsRef<Path>,
    json_path: &[&str],
    patch: &str,
) -> Result<(), BoxError> {
    use edikt_core::{Document, Step};
    let file_path = file_path.as_ref();
    let input = std::fs::read_to_string(&file_path).unwrap_or_else(|_| "{}".to_string());
    let mut cst = edikt_jsonc::parse(&input)?;
    cst.set(
        json_path
            .into_iter()
            .map(ToString::to_string)
            .map(Step::Field)
            .collect::<Vec<Step>>()
            .as_slice(),
        &edikt_jsonc::parse(patch)?.to_value(),
    )?;
    let output = cst.to_source();
    std::fs::write(&file_path, output)?;
    Ok(())
}

#[cfg(false)]
fn patch_jsonc_file_with_jsonc_parser(
    file_path: impl AsRef<Path>,
    json_path: &[&str],
    _patch: &str,
) -> Result<(), BoxError> {
    use jsonc_parser::cst::CstRootNode;
    let file_path = file_path.as_ref();
    let input = std::fs::read_to_string(&file_path).unwrap_or_else(|_| "{}".to_string());
    let cst = CstRootNode::parse(&input, &Default::default())?;
    let mut cursor = cst.object_value_or_set();
    for key in json_path {
        cursor = cursor.object_value_or_set(key);
    }
    //cursor.replace_with(/* ...?... */); // TODO: how to parse patch into a CstInputValue?
    let output = cst.to_string();
    std::fs::write(&file_path, output)?;
    Ok(())
}
