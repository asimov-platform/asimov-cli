// This is free and unencumbered software released into the public domain.

use clientele::SysexitsError::*;
use indoc::indoc;
use std::process::{Command, Stdio};
use temp_dir::TempDir;

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

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
