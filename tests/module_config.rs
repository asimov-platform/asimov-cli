// This is free and unencumbered software released into the public domain.

//! Tests for `asimov module config`, covering the guarantees that callers and
//! agents rely on: secrets are not disclosed, stored values are private, a
//! rejected batch changes nothing, values resolve in the documented order, and
//! nothing ever blocks waiting for input that cannot arrive.

use clientele::SysexitsError::*;
use indoc::indoc;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use temp_dir::TempDir;

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

/// The environment variable that the `host` variable reads from.
const HOST_ENV: &str = "ASIMOV_TEST_MODULE_CONFIG_HOST";

const MANIFEST: &str = indoc! {r#"
    {
      "name": "demo",
      "config": {
        "variables": [
          { "name": "api-key", "secret": true },
          {
            "name": "host",
            "environment": "ASIMOV_TEST_MODULE_CONFIG_HOST",
            "default_value": "default.example"
          }
        ]
      }
    }
"#};

struct Sandbox(TempDir);

struct Run {
    code: i32,
    stdout: String,
}

impl Sandbox {
    fn new() -> Result<Self> {
        Self::with_manifest(MANIFEST)
    }

    fn with_manifest(manifest: &str) -> Result<Self> {
        let dir = TempDir::new()?;
        let module_dir = dir.child("modules/installed/demo");
        std::fs::create_dir_all(&module_dir)?;
        std::fs::write(module_dir.join("manifest.json"), manifest)?;
        Ok(Self(dir))
    }

    fn root(&self) -> &Path {
        self.0.path()
    }

    fn value_file(&self, key: &str) -> PathBuf {
        self.root()
            .join("configs")
            .join("default")
            .join("demo")
            .join(key)
    }

    fn run(&self, args: &[&str]) -> Result<Run> {
        self.run_with_env(args, None)
    }

    fn run_with_env(&self, args: &[&str], host: Option<&str>) -> Result<Run> {
        let mut command = Command::new(env!("CARGO_BIN_EXE_asimov"));
        command
            .args(["module", "config"])
            .args(args)
            .env("ASIMOV_ROOT", self.root())
            // never inherit a terminal: a test must fail rather than block
            .stdin(Stdio::null());

        match host {
            Some(value) => command.env(HOST_ENV, value),
            None => command.env_remove(HOST_ENV),
        };

        let output = command.output()?;
        Ok(Run {
            code: output.status.code().expect("should exit normally"),
            stdout: String::from_utf8(output.stdout)?,
        })
    }
}

/// Variable names are joined onto the configuration directory, so `unset` must
/// not accept anything that resolves outside of it.
#[test]
fn unset_cannot_remove_files_outside_the_configuration_directory() -> Result {
    let sandbox = Sandbox::new()?;

    let victim = sandbox.root().join("victim");
    std::fs::write(&victim, "keep me")?;

    for key in [
        victim.to_str().expect("path should be UTF-8"),
        "../../victim",
    ] {
        let run = sandbox.run(&["unset", "demo", key])?;
        assert_eq!(run.code, EX_USAGE as i32, "should reject `{key}`");
        assert!(victim.exists(), "`{key}` removed a file outside the config");
    }

    Ok(())
}

/// The manifest is the other way a bad name reaches a path, and it is not
/// covered by the check that rejects undeclared keys.
#[test]
fn a_manifest_declaring_an_unusable_variable_name_is_rejected() -> Result {
    let sandbox = Sandbox::with_manifest(indoc! {r#"
        {
          "name": "demo",
          "config": { "variables": [{ "name": "../escape" }] }
        }
    "#})?;

    let escape = sandbox.root().join("escape");
    std::fs::write(&escape, "keep me")?;

    for args in [
        vec!["show", "demo"],
        vec!["set", "demo", "../escape=value"],
        vec!["unset", "demo", "--all"],
    ] {
        let run = sandbox.run(&args)?;
        assert_eq!(run.code, EX_DATAERR as i32, "should reject {args:?}");
    }

    assert_eq!(std::fs::read_to_string(&escape)?, "keep me");

    Ok(())
}

/// Secret values may be read by name, but must not appear in output that the
/// caller did not ask to contain them.
#[test]
fn secret_values_are_shown_only_when_read_by_name() -> Result {
    let sandbox = Sandbox::new()?;
    sandbox.run(&["set", "demo", "api-key=s3cret-value"])?;

    let shown = sandbox.run(&["show", "demo"])?;
    assert!(
        !shown.stdout.contains("s3cret-value"),
        "`show` disclosed a secret: {}",
        shown.stdout
    );
    assert!(shown.stdout.contains("api-key"), "`show` omitted the name");

    let got = sandbox.run(&["get", "demo", "api-key"])?;
    assert_eq!(got.stdout.trim(), "s3cret-value");

    Ok(())
}

/// A rejected assignment must not leave part of the batch applied.
#[test]
fn a_rejected_batch_changes_nothing() -> Result {
    let sandbox = Sandbox::new()?;
    sandbox.run(&["set", "demo", "host=first"])?;

    let run = sandbox.run(&["set", "demo", "host=second", "nonexistent=value"])?;
    assert_eq!(run.code, EX_USAGE as i32);

    let host = sandbox.run(&["get", "demo", "host", "--stored"])?;
    assert_eq!(host.stdout.trim(), "first", "a rejected batch was applied");

    Ok(())
}

/// Stored values are frequently credentials, so neither they nor the directory
/// holding them may be readable by other users.
#[cfg(unix)]
#[test]
fn stored_values_are_private_to_the_user() -> Result {
    use std::os::unix::fs::PermissionsExt;

    let sandbox = Sandbox::new()?;
    sandbox.run(&["set", "demo", "api-key=s3cret-value"])?;

    let mode =
        |path: &Path| -> Result<u32> { Ok(std::fs::metadata(path)?.permissions().mode() & 0o777) };

    assert_eq!(mode(&sandbox.value_file("api-key"))?, 0o600);
    assert_eq!(mode(&sandbox.root().join("configs/default/demo"))?, 0o700);

    Ok(())
}

/// `get` reports what a module will actually receive, which means resolving in
/// the same order the SDK does.
#[test]
fn get_resolves_the_environment_then_the_stored_value_then_the_default() -> Result {
    let sandbox = Sandbox::new()?;

    let run = sandbox.run(&["get", "demo", "host"])?;
    assert_eq!(run.stdout.trim(), "default.example");

    sandbox.run(&["set", "demo", "host=stored.example"])?;
    let run = sandbox.run(&["get", "demo", "host"])?;
    assert_eq!(run.stdout.trim(), "stored.example");

    let run = sandbox.run_with_env(&["get", "demo", "host"], Some("env.example"))?;
    assert_eq!(run.stdout.trim(), "env.example");

    // `--stored` answers a different question, and ignores the environment
    let run = sandbox.run_with_env(&["get", "demo", "host", "--stored"], Some("env.example"))?;
    assert_eq!(run.stdout.trim(), "stored.example");

    Ok(())
}

/// Interactive setup is the one subcommand that prompts; without a terminal it
/// has to fail, because waiting would hang a script or an agent forever.
#[test]
fn setup_without_a_terminal_fails_rather_than_waiting() -> Result {
    let sandbox = Sandbox::new()?;

    let run = sandbox.run(&["setup", "demo"])?;
    assert_eq!(run.code, EX_UNAVAILABLE as i32);

    Ok(())
}
