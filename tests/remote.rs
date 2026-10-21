// This is free and unencumbered software released into the public domain.

#![cfg(feature = "remote")]

extern crate alloc;

use alloc::{format, string::String};
use asimov_directory::fs::StateDirectory;
use clientele::SysexitsError::{self, *};
use std::process::{Command, Output, Stdio};

#[cfg(unix)]
#[path = "remote/check.rs"]
mod check;

struct Sandbox {
    home: temp_dir::TempDir,
    state: StateDirectory,
}

impl Sandbox {
    fn new() -> Self {
        let home = temp_dir::TempDir::new().unwrap();
        let home_state = StateDirectory::open(home.path()).unwrap();
        let state = StateDirectory::open(home_state.join("custom-state")).unwrap();
        Self { home, state }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_asimov"));
        command
            .current_dir(self.home.path())
            .env("HOME", self.home.path())
            .env("USERPROFILE", self.home.path())
            .env("ASIMOV_ROOT", self.state.as_str())
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
fn remote_lifecycle_and_aliases_use_the_selected_state_directory() {
    let sandbox = Sandbox::new();
    assert!(sandbox.run(&["remote", "list"], EX_OK).stdout.is_empty());
    assert!(!sandbox.state.join("remotes").exists());
    for (name, url) in [
        ("origin", "https://example.com"),
        ("local", "http://localhost:8080/api"),
    ] {
        assert!(
            sandbox
                .run(&["remote", "add", name, url], EX_OK)
                .stdout
                .is_empty()
        );
        assert_eq!(
            std::fs::read_to_string(sandbox.state.join(format!("remotes/{name}.uri"))).unwrap(),
            format!("{url}\n")
        );
    }
    assert_eq!(
        sandbox.run(&["remote", "ls"], EX_OK).stdout,
        b"local\norigin\n"
    );
    assert_eq!(
        sandbox.run(&["--verbose", "remote", "list"], EX_OK).stdout,
        b"local\thttp://localhost:8080/api\norigin\thttps://example.com\n",
    );
    sandbox.run(&["remote", "rename", "origin", "upstream"], EX_OK);
    sandbox.run(&["remote", "mv", "upstream", "backup"], EX_OK);
    assert_eq!(
        std::fs::read_to_string(sandbox.state.join("remotes/backup.uri")).unwrap(),
        "https://example.com\n"
    );
    sandbox.run(&["remote", "remove", "backup"], EX_OK);
    sandbox.run(&["remote", "rm", "local"], EX_OK);
    assert!(sandbox.run(&["remote", "list"], EX_OK).stdout.is_empty());
    for name in ["modules", "snapshots"] {
        assert!(!sandbox.state.join(name).exists());
    }
}

#[test]
fn failures_preserve_existing_remotes_and_report_exit_codes() {
    let sandbox = Sandbox::new();
    sandbox.run(
        &["remote", "add", "../escape", "https://example.com"],
        EX_USAGE,
    );
    sandbox.run(&["remote", "add", "origin", "iroh://example.com"], EX_USAGE);
    sandbox.run(&["remote", "add", "origin", "not-a-url"], EX_USAGE);
    assert!(!sandbox.state.join("remotes").exists());
    sandbox.run(&["remote", "add", "origin", "https://example.com"], EX_OK);
    sandbox.run(
        &["remote", "add", "origin", "http://other.example"],
        EX_CANTCREAT,
    );
    sandbox.run(&["remote", "add", "backup", "http://other.example"], EX_OK);
    sandbox.run(&["remote", "rename", "origin", "backup"], EX_CANTCREAT);
    sandbox.run(&["remote", "rename", "origin", "../escape"], EX_USAGE);
    sandbox.run(&["remote", "rename", "absent", "new"], EX_NOINPUT);
    sandbox.run(&["remote", "remove", "absent"], EX_NOINPUT);
    assert_eq!(
        std::fs::read_to_string(sandbox.state.join("remotes/origin.uri")).unwrap(),
        "https://example.com\n"
    );
    assert_eq!(
        std::fs::read_to_string(sandbox.state.join("remotes/backup.uri")).unwrap(),
        "http://other.example\n"
    );
    assert!(!sandbox.state.join("escape").exists());
    for contents in [
        "https://example.com",
        "https://example.com\r\n",
        "# comment\nhttps://example.com\n",
        "https://example.com\nhttps://other.example\n",
    ] {
        std::fs::write(sandbox.state.join("remotes/origin.uri"), contents).unwrap();
        sandbox.run(&["remote", "list"], EX_DATAERR);
    }
    sandbox.run(&["remote", "remove", "origin"], EX_OK);
}

#[cfg(unix)]
#[test]
fn remotes_default_to_the_home_state_directory() {
    let sandbox = Sandbox::new();
    let output = sandbox
        .command()
        .env_remove("ASIMOV_ROOT")
        .args(["remote", "add", "origin", "https://example.com"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let home = StateDirectory::open(sandbox.home.path()).unwrap();
    let state = StateDirectory::open(home.join(".asimov")).unwrap();
    assert_eq!(
        std::fs::read_to_string(state.join("remotes/origin.uri")).unwrap(),
        "https://example.com\n"
    );
    assert!(!sandbox.state.join("remotes").exists());
}

#[test]
fn help_and_checks_for_unknown_remotes_report_correctly() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&["remote", "--help"], EX_OK);
    let help = String::from_utf8(output.stdout).unwrap();
    for command in ["list", "add", "rename", "remove", "check"] {
        assert!(help.contains(command));
    }
    sandbox.run(&["remote", "check", "origin"], EX_NOINPUT);
    sandbox.run(&["remote", "ping", "origin"], EX_NOINPUT);
}
