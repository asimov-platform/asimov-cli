// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions};
use clap::ValueEnum;
use std::path::Path;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ProxyInstallTarget {
    /// Cursor (<https://cursor.com>).
    #[cfg(feature = "unstable")]
    Cursor,

    /// Obsidian (<https://obsidian.md>).
    #[cfg(feature = "unstable")]
    Obsidian,

    /// Visual Studio Code (<https://code.visualstudio.com>).
    #[cfg(feature = "unstable")]
    VSCode,

    /// Zed (<https://zed.dev>).
    Zed,
}

pub async fn install(
    mut apps: Vec<ProxyInstallTarget>,
    flags: &StandardOptions,
) -> Result<(), BoxError> {
    use ProxyInstallTarget::*;
    let home_path = dirs::home_dir().expect("HOME should be set");
    if apps.is_empty() {
        apps.push(Zed);
    }
    for app in apps {
        install_app(app, &home_path, flags).await?;
    }
    Ok(())
}

pub async fn install_app(
    app: ProxyInstallTarget,
    home_path: &Path,
    flags: &StandardOptions,
) -> Result<(), BoxError> {
    use ProxyInstallTarget::*;
    match app {
        #[cfg(feature = "unstable")]
        Cursor | Obsidian | VSCode => {
            eprintln!("error: proxy installation for {app:?} is not supported yet");
            return Err(crate::SysexitsError::EX_UNAVAILABLE.into());
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
    use std::io::Write;
    // Preserve symlinked settings files by replacing their resolved target.
    let file_path = std::fs::canonicalize(file_path)?;
    let permissions = std::fs::metadata(&file_path)?.permissions();
    let input = std::fs::read_to_string(&file_path)?;
    // The editing parser recovers from malformed syntax; reject it first.
    let parsed = jsonc_parser::parse_to_value(&input, &Default::default())?;
    if !matches!(parsed, Some(jsonc_parser::JsonValue::Object(_))) {
        return Err("application config must be a JSON object".into());
    }
    let mut cst = edikt_jsonc::parse(&input)?;
    cst.set(
        json_path
            .iter()
            .map(ToString::to_string)
            .map(Step::Field)
            .collect::<Vec<Step>>()
            .as_slice(),
        &edikt_jsonc::parse(patch)?.to_value(),
    )?;
    let output = cst.to_source();
    crate::shared::atomic_write(&file_path, |file| {
        file.write_all(output.as_bytes())?;
        file.set_permissions(permissions)
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "unstable")]
    #[tokio::test]
    async fn unfinished_targets_return_unavailable() {
        #[derive(clap::Parser)]
        struct Args {
            #[command(flatten)]
            flags: StandardOptions,
        }
        let flags = <Args as clap::Parser>::parse_from(["test"]).flags;
        let root = temp_dir::TempDir::new().unwrap();
        for target in [
            ProxyInstallTarget::Cursor,
            ProxyInstallTarget::Obsidian,
            ProxyInstallTarget::VSCode,
        ] {
            let error = install_app(target, root.path(), &flags).await.unwrap_err();
            assert_eq!(
                error.downcast_ref::<crate::SysexitsError>(),
                Some(&crate::SysexitsError::EX_UNAVAILABLE)
            );
        }
    }

    #[test]
    fn invalid_configs_are_not_overwritten() -> Result<(), BoxError> {
        let root = temp_dir::TempDir::new()?;
        let path = root.child("settings.json");
        for input in [b"\xff\xfe".as_slice(), b"{broken json", b"", b"null", b"[]"] {
            std::fs::write(&path, input)?;
            assert!(patch_jsonc_file_with_edikt(&path, &["provider"], "{}").is_err());
            assert_eq!(std::fs::read(&path)?, input);
        }
        std::fs::remove_file(&path)?;
        assert!(patch_jsonc_file_with_edikt(&path, &["provider"], "{}").is_err());
        assert!(!path.exists());
        std::fs::create_dir(&path)?;
        assert!(patch_jsonc_file_with_edikt(&path, &["provider"], "{}").is_err());
        assert!(path.is_dir());
        Ok(())
    }

    #[test]
    fn patching_preserves_comments_and_unrelated_settings() -> Result<(), BoxError> {
        let root = temp_dir::TempDir::new()?;
        let path = root.child("settings.json");
        std::fs::write(&path, "{\n  // keep this comment\n  \"font_size\": 14\n}\n")?;
        for _ in 0..2 {
            patch_jsonc_file_with_edikt(&path, &["provider"], r#"{"name":"ASIMOV"}"#)?;
            let output = std::fs::read_to_string(&path)?;
            assert!(output.contains("// keep this comment"));
            assert!(output.contains("\"font_size\": 14"));
            assert_eq!(output.matches("ASIMOV").count(), 1);
        }
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn atomic_patching_preserves_symlinks_and_permissions() -> Result<(), BoxError> {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let root = temp_dir::TempDir::new()?;
        let target = root.child("target.json");
        let link = root.child("settings.json");
        std::fs::write(&target, "{}")?;
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640))?;
        symlink(&target, &link)?;
        patch_jsonc_file_with_edikt(&link, &["provider"], r#"{"name":"ASIMOV"}"#)?;
        assert!(std::fs::symlink_metadata(&link)?.is_symlink());
        assert!(std::fs::read_to_string(&target)?.contains("ASIMOV"));
        assert_eq!(
            std::fs::metadata(&target)?.permissions().mode() & 0o777,
            0o640
        );
        assert_eq!(std::fs::read_dir(root.path())?.count(), 2);
        Ok(())
    }
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
