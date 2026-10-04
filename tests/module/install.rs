// This is free and unencumbered software released into the public domain.

use clientele::SysexitsError::*;
use indoc::indoc;
use std::process::{Command, Stdio};
use temp_dir::TempDir;

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

#[cfg(unix)]
#[test]
fn install_reports_registry_state_errors() -> Result {
    for state in ["installed", "enabled"] {
        let root = TempDir::new()?;
        std::fs::create_dir_all(root.child("modules/installed"))?;
        if state == "enabled" {
            let installed = root.child("modules/installed/demo");
            std::fs::create_dir_all(&installed)?;
            std::fs::write(installed.join("manifest.json"), r#"{"name":"demo"}"#)?;
            std::os::unix::fs::symlink("enabled", root.child("modules/enabled"))?;
        } else {
            std::os::unix::fs::symlink("demo", root.child("modules/installed/demo"))?;
        }
        let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
            .args(["module", "install", "demo"])
            .env("ASIMOV_ROOT", root.path())
            .stdin(Stdio::null())
            .output()?;
        assert_eq!(
            output.status.code(),
            Some(EX_IOERR as i32),
            "{state}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn install_enables_a_module_with_an_unset_optional_variable() -> Result {
    let root = TempDir::new()?;
    let module_dir = root.child("modules/installed/demo");
    std::fs::create_dir_all(&module_dir)?;
    std::fs::write(
        module_dir.join("manifest.json"),
        indoc! {r#"
            {
              "name": "demo",
              "config": {
                "variables": [{ "name": "optional", "optional": true }]
              }
            }
        "#},
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
        .args(["module", "install", "demo"])
        .env("ASIMOV_ROOT", root.path())
        .stdin(Stdio::null())
        .output()?;

    assert_eq!(output.status.code(), Some(EX_OK as i32));
    assert!(root.child("modules/enabled/demo").try_exists()?);

    Ok(())
}
