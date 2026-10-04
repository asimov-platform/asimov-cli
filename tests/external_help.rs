// This is free and unencumbered software released into the public domain.

#![cfg(unix)]

use std::{
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};

#[test]
fn nested_external_help_places_the_help_flag_after_subcommands()
-> Result<(), Box<dyn core::error::Error>> {
    let root = temp_dir::TempDir::new()?;
    let program = root.child("asimov-probe");
    std::fs::write(&program, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n")?;
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))?;
    for (args, expected) in [
        (vec!["help", "probe"], "--help\n"),
        (
            vec!["help", "probe", "nested", "child"],
            "nested\nchild\n--help\n",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
            .args(args)
            .env("PATH", root.path())
            .env("ASIMOV_ROOT", root.path())
            .current_dir(root.path())
            .stdin(Stdio::null())
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout)?, expected);
    }
    Ok(())
}
