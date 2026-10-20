// This is free and unencumbered software released into the public domain.

#![cfg(feature = "resource")]

extern crate alloc;

use alloc::string::String;
use std::process::{Command, Stdio};
use temp_dir::TempDir;

fn command(home: &TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_asimov"));
    command
        .arg("--color=never")
        .current_dir(home.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("ASIMOV_ROOT", home.child("state"))
        .env("ASIMOV_KEYRING_BACKEND", "file")
        .env("DO_NOT_TRACK", "1")
        .env("PATH", home.path())
        .stdin(Stdio::null());
    command
}

#[test]
fn snapshot_visibility_requires_unstable() {
    let home = TempDir::new().unwrap();
    for group in ["resource", "source"] {
        let output = command(&home).args([group, "--help"]).output().unwrap();
        assert!(output.status.success(), "{output:?}");
        let help = String::from_utf8(output.stdout).unwrap();
        assert_eq!(
            help.lines()
                .any(|line| line.split_whitespace().next() == Some("snap")),
            cfg!(feature = "unstable"),
            "{help}"
        );
        let output = command(&home)
            .args([group, "snap", "--help"])
            .output()
            .unwrap();
        if cfg!(feature = "unstable") {
            assert!(output.status.success(), "{output:?}");
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("Usage: asimov resource snap")
            );
        } else {
            assert_eq!(output.status.code(), Some(2), "{output:?}");
            assert!(String::from_utf8_lossy(&output.stderr).contains("unrecognized subcommand"));
        }
    }
    let output = command(&home).arg("--help").output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let help = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        help.contains("asimov resource snap"),
        cfg!(feature = "unstable"),
        "{help}"
    );
    assert!(!home.child("state").exists());
}

#[test]
#[cfg(feature = "unstable")]
fn snapshot_aliases_dispatch_and_initialize_storage() {
    for path in [
        alloc::vec!["resource", "snap"],
        alloc::vec!["source", "snap"],
        alloc::vec!["snap"],
    ] {
        for operation in [
            None,
            Some("save"),
            Some("create"),
            Some("update"),
            Some("list"),
            Some("compact"),
        ] {
            let home = TempDir::new().unwrap();
            let output = command(&home).args(&path).args(operation).output().unwrap();
            assert!(
                output.status.success(),
                "{path:?} {operation:?}: {output:?}"
            );
            assert!(output.stdout.is_empty(), "{output:?}");
            assert!(home.child("state/modules/installed").is_dir());
            assert!(home.child("state/snapshots").is_dir());
        }
    }
}
