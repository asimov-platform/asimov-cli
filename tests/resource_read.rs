// This is free and unencumbered software released into the public domain.

#![cfg(all(unix, feature = "resource"))]

use clientele::SysexitsError::*;
use serde_json::json;
use std::{
    os::unix::fs::{PermissionsExt, symlink},
    process::{Command, Stdio},
};
use temp_dir::TempDir;

#[test]
fn reader_failures_preserve_sysexits() {
    for (exit, expected) in [
        (0, EX_OK),
        (65, EX_DATAERR),
        (66, EX_NOINPUT),
        (1, EX_UNAVAILABLE),
    ] {
        let root = TempDir::new().unwrap();
        let installed = root.child(".asimov/modules/installed/local-test");
        std::fs::create_dir_all(&installed).unwrap();
        std::fs::write(
            installed.join("manifest.json"),
            json!({
                "name": "local-test",
                "provides": { "programs": ["asimov-local-test-reader"] },
                "handles": { "url_prefixes": ["https://local.example/"] },
            })
            .to_string(),
        )
        .unwrap();
        std::fs::create_dir_all(root.child(".asimov/modules/enabled")).unwrap();
        symlink(&installed, root.child(".asimov/modules/enabled/local-test")).unwrap();
        std::fs::create_dir_all(root.child("bin")).unwrap();
        let program = root.child("bin/asimov-local-test-reader");
        std::fs::write(&program, format!("#!/bin/sh\nexit {exit}\n")).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
            .args(["read", "https://local.example/data"])
            .current_dir(root.path())
            .env("HOME", root.path())
            .env("ASIMOV_ROOT", root.child(".asimov"))
            .env("ASIMOV_KEYRING_BACKEND", "file")
            .env("DO_NOT_TRACK", "1")
            .env("PATH", root.child("bin"))
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected as i32),
            "exit {exit}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
