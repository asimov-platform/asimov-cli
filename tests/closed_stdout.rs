// This is free and unencumbered software released into the public domain.

#![cfg(all(unix, any(feature = "remote", feature = "resource")))]

use std::process::{Command, Stdio};
use temp_dir::TempDir;

/// Runs `asimov <args>` in `home`, checking the exit status and stderr.
///
/// With `closed_stdout`, stdout is connected to a pipe whose reader has
/// already exited, as when piping into `head -1`; output must stop quietly.
fn run(home: &TempDir, args: &[&str], closed_stdout: bool) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_asimov"));
    command
        .args(args)
        .current_dir(home.path())
        .env("HOME", home.path())
        .env("ASIMOV_ROOT", home.child(".asimov"))
        .env("ASIMOV_KEYRING_BACKEND", "file")
        .env("DO_NOT_TRACK", "1")
        .env("PATH", home.child("bin"))
        .env_remove("ASIMOV_RELAY_URL")
        .stdin(Stdio::null());
    if closed_stdout {
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        command.stdout(writer);
    }
    let output = command.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{args:?}: {stderr}");
    assert!(stderr.is_empty(), "{args:?}: {stderr}");
}

#[test]
#[cfg(feature = "resource")]
fn resource_output_stops_quietly() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let home = TempDir::new().unwrap();
    let installed = home.child(".asimov/modules/installed/local-test");
    std::fs::create_dir_all(&installed).unwrap();
    std::fs::write(
        installed.join("manifest.json"),
        r#"{
          "name": "local-test",
          "provides": {
            "programs": ["asimov-local-test-fetcher", "asimov-local-test-lister"]
          },
          "handles": { "url_prefixes": ["https://local.example/"] }
        }"#,
    )
    .unwrap();
    std::fs::create_dir_all(home.child(".asimov/modules/enabled")).unwrap();
    symlink(&installed, home.child(".asimov/modules/enabled/local-test")).unwrap();
    std::fs::create_dir_all(home.child("bin")).unwrap();
    for program in ["fetcher", "lister"] {
        let program = home.child(format!("bin/asimov-local-test-{program}"));
        std::fs::write(&program, "#!/bin/sh\necho '{\"n\":1}'\necho '{\"n\":2}'\n").unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let urls = ["https://a.example/", "https://b.example/"];
    run(&home, &[&["resource", "track"][..], &urls].concat(), false);
    run(&home, &["resource", "list"], true);
    run(&home, &["fetch", "https://local.example/data"], true);
    run(
        &home,
        &["fetch", "--jq=.n", "https://local.example/data"],
        true,
    );
    run(&home, &["list", "https://local.example/"], true);
    run(&home, &["list", "--jq=.n", "https://local.example/"], true);
}

#[test]
#[cfg(feature = "remote")]
fn remote_output_stops_quietly() {
    let home = TempDir::new().unwrap();
    std::fs::create_dir_all(home.child("bin")).unwrap();
    run(
        &home,
        &["remote", "add", "origin", "https://a.example"],
        false,
    );
    run(&home, &["remote", "list"], true);
    run(&home, &["remote", "list", "-v"], true);
}
