// This is free and unencumbered software released into the public domain.

#![cfg(feature = "resource")]

extern crate alloc;

use alloc::string::String;
use clientele::SysexitsError::{self, *};
use std::{
    path::PathBuf,
    process::{Command, Output, Stdio},
    time::{Duration, UNIX_EPOCH},
};

struct Sandbox {
    home: temp_dir::TempDir,
    root: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let home = temp_dir::TempDir::new().unwrap();
        let root = home.child("custom-state");
        std::fs::create_dir(&root).unwrap();
        Self { home, root }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_asimov"));
        command
            .current_dir(self.home.path())
            .env("HOME", self.home.path())
            .env("USERPROFILE", self.home.path())
            .env("ASIMOV_ROOT", &self.root)
            .env("ASIMOV_KEYRING_BACKEND", "file")
            .env("DO_NOT_TRACK", "1")
            .env("PATH", self.home.path())
            .stdin(Stdio::null());
        command
    }

    fn run(&self, args: &[&str], expected: SysexitsError) -> Output {
        let output = self.command().args(args).output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected as i32),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr),
        );
        if expected != EX_OK {
            assert!(!output.stderr.is_empty(), "{args:?}");
            assert!(output.stdout.is_empty(), "{args:?}");
        }
        output
    }
}

#[test]
fn tracking_lifecycle_and_aliases_preserve_url_order() {
    let sandbox = Sandbox::new();
    assert!(sandbox.run(&["resource", "list"], EX_OK).stdout.is_empty());
    sandbox.run(&["resource", "untrack", "https://absent.example"], EX_OK);
    assert!(!sandbox.root.join("resources").exists());

    assert!(
        sandbox
            .run(
                &[
                    "resource",
                    "track",
                    "HTTPS://B.example",
                    "https://a.example",
                    "HTTPS://B.example"
                ],
                EX_OK
            )
            .stdout
            .is_empty()
    );
    sandbox.run(
        &[
            "source",
            "add",
            "https://a.example",
            "https://c.example",
            "https://d.example",
        ],
        EX_OK,
    );
    let expected = b"HTTPS://B.example\nhttps://a.example\nhttps://c.example\nhttps://d.example\n";
    let path = sandbox.root.join("resources/tracked.uris");
    assert_eq!(std::fs::read(&path).unwrap(), expected);
    for args in [&["resource", "ls"][..], &["source", "dir"], &["list"]] {
        assert_eq!(sandbox.run(args, EX_OK).stdout, expected);
    }

    let file = std::fs::File::options().write(true).open(&path).unwrap();
    file.set_modified(UNIX_EPOCH + Duration::from_secs(1234567890))
        .unwrap();
    let before = std::fs::metadata(&path).unwrap().modified().unwrap();
    sandbox.run(&["resource", "track", "https://a.example"], EX_OK);
    sandbox.run(&["resource", "rm", "https://absent.example"], EX_OK);
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        before
    );
    sandbox.run(
        &[
            "resource",
            "remove",
            "https://a.example",
            "HTTPS://B.example",
            "https://absent.example",
        ],
        EX_OK,
    );
    assert_eq!(
        sandbox.run(&["list"], EX_OK).stdout,
        b"https://c.example\nhttps://d.example\n"
    );
    sandbox.run(
        &[
            "resource",
            "untrack",
            "https://c.example",
            "https://d.example",
        ],
        EX_OK,
    );
    assert!(sandbox.run(&["list"], EX_OK).stdout.is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), b"");
    assert_eq!(
        std::fs::read_dir(sandbox.root.join("resources"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn invalid_batches_and_corrupt_storage_fail_without_partial_changes() {
    let sandbox = Sandbox::new();
    for invalid in ["relative/path", "https://a.example\nhttps://b.example", ""] {
        sandbox.run(
            &["resource", "track", "https://valid.example", invalid],
            EX_USAGE,
        );
    }
    assert!(!sandbox.root.join("resources").exists());
    sandbox.run(&["resource", "track", "https://valid.example"], EX_OK);
    sandbox.run(
        &["resource", "untrack", "https://valid.example", "not-a-url"],
        EX_USAGE,
    );
    assert_eq!(
        sandbox.run(&["list"], EX_OK).stdout,
        b"https://valid.example\n"
    );
    let path = sandbox.root.join("resources/tracked.uris");
    let malformed = b"https://valid.example\nnot a URL\n";
    std::fs::write(&path, malformed).unwrap();
    let output = sandbox
        .command()
        .args(["resource", "list"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(EX_DATAERR as i32));
    assert_eq!(output.stdout, b"https://valid.example\n");
    assert!(!output.stderr.is_empty());
    sandbox.run(&["resource", "track", "https://new.example"], EX_DATAERR);
    sandbox.run(
        &["resource", "untrack", "https://valid.example"],
        EX_DATAERR,
    );
    assert_eq!(std::fs::read(&path).unwrap(), malformed);
}

#[cfg(unix)]
#[test]
fn default_storage_is_under_the_home_state_directory() {
    let sandbox = Sandbox::new();
    let output = sandbox
        .command()
        .env_remove("ASIMOV_ROOT")
        .args(["resource", "track", "https://例え.テスト/🦀#片"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(sandbox.home.child(".asimov/resources/tracked.uris")).unwrap(),
        "https://例え.テスト/🦀#片\n".as_bytes()
    );
    assert!(!sandbox.root.join("resources").exists());
}

#[cfg(unix)]
#[test]
fn symlinked_storage_cannot_expose_or_modify_other_state() {
    let sandbox = Sandbox::new();
    let private = sandbox.root.join("private");
    std::fs::create_dir(&private).unwrap();
    std::fs::write(private.join("tracked.uris"), "https://private.example\n").unwrap();
    std::os::unix::fs::symlink(&private, sandbox.root.join("resources")).unwrap();
    for args in [
        &["resource", "list"][..],
        &["resource", "track", "https://new.example"],
        &["resource", "untrack", "https://private.example"],
    ] {
        let output = sandbox.command().args(args).output().unwrap();
        assert!(!output.status.success(), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(!output.stderr.is_empty(), "{args:?}");
    }
    assert_eq!(
        std::fs::read(private.join("tracked.uris")).unwrap(),
        b"https://private.example\n"
    );
}
