// This is free and unencumbered software released into the public domain.

#![cfg(unix)]

use std::{
    ffi::OsString,
    os::unix::{ffi::OsStringExt, fs::PermissionsExt},
    process::{Command, Stdio},
};

#[test]
fn external_arguments_preserve_non_utf8_and_empty_values() -> Result<(), Box<dyn core::error::Error>>
{
    let root = temp_dir::TempDir::new()?;
    let program = root.child("asimov-probe");
    std::fs::write(&program, "#!/bin/sh\nprintf '%s\\0' \"$@\"\n")?;
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))?;
    let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
        .arg("probe")
        .arg(OsString::from_vec(b"non\xffpath".to_vec()))
        .args(["", "two words", "--flag"])
        .env("ASIMOV_ROOT", root.path())
        .env("PATH", root.path())
        .current_dir(root.path())
        .stdin(Stdio::null())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"non\xffpath\0\0two words\0--flag\0");
    Ok(())
}
