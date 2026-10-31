// This is free and unencumbered software released into the public domain.

#![cfg(unix)]

use std::{
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};

#[test]
fn external_exit_statuses_are_preserved() -> Result<(), Box<dyn core::error::Error>> {
    let root = temp_dir::TempDir::new()?;
    for (name, script, expected) in [
        ("asimov-fail", "exit 42", 42),
        ("asimov-sigterm", "kill -TERM $$", 128 + 15),
    ] {
        let program = root.child(name);
        std::fs::write(&program, format!("#!/bin/sh\n{script}\n"))?;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))?;
        let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
            .arg(name.trim_start_matches("asimov-"))
            .env("ASIMOV_ROOT", root.path())
            .env("ASIMOV_KEYRING_BACKEND", "file")
            .env("DO_NOT_TRACK", "1")
            .env("HOME", root.path())
            .env("PATH", root.path())
            .current_dir(root.path())
            .stdin(Stdio::null())
            .output()?;
        assert_eq!(
            output.status.code(),
            Some(expected),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
