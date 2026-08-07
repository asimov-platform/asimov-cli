// This is free and unencumbered software released into the public domain.

use clientele::SysexitsError::*;
use indoc::indoc;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use temp_dir::TempDir;

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

/// The environment variables that the fixture's variables read from.
const KEY_ENV: &str = "ASIMOV_TEST_MODULE_CONFIG_KEY";
const HOST_ENV: &str = "ASIMOV_TEST_MODULE_CONFIG_HOST";

const MANIFEST: &str = indoc! {r#"
    {
      "name": "demo",
      "config": {
        "variables": [
          {
            "name": "api-key",
            "secret": true,
            "environment": "ASIMOV_TEST_MODULE_CONFIG_KEY"
          },
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

    /// Runs `asimov module config <args>`.
    fn config(&self, args: &[&str]) -> Result<Run> {
        self.config_env(args, &[])
    }

    fn config_env(&self, args: &[&str], env: &[(&str, &str)]) -> Result<Run> {
        let mut all = vec!["config"];
        all.extend_from_slice(args);
        self.module_env(&all, env)
    }

    /// Runs `asimov module <args>`.
    fn module(&self, args: &[&str]) -> Result<Run> {
        self.module_env(args, &[])
    }

    fn module_env(&self, args: &[&str], env: &[(&str, &str)]) -> Result<Run> {
        let mut command = Command::new(env!("CARGO_BIN_EXE_asimov"));
        command
            .arg("module")
            .args(args)
            .env("ASIMOV_ROOT", self.root())
            // start from a known environment, whatever the developer's shell has
            .env_remove(KEY_ENV)
            .env_remove(HOST_ENV)
            .envs(env.iter().copied())
            // never inherit a terminal: a test must fail rather than block
            .stdin(Stdio::null());

        let output = command.output()?;
        Ok(Run {
            code: output.status.code().expect("should exit normally"),
            stdout: String::from_utf8(output.stdout)?,
        })
    }
}

#[test]
fn unset_cannot_remove_files_outside_the_configuration_directory() -> Result {
    let sandbox = Sandbox::new()?;

    let victim = sandbox.root().join("victim");
    std::fs::write(&victim, "keep me")?;

    for key in [
        victim.to_str().expect("path should be UTF-8"),
        "../../victim",
    ] {
        let run = sandbox.config(&["unset", "demo", key])?;
        assert_eq!(run.code, EX_USAGE as i32, "should reject `{key}`");
        assert!(victim.exists(), "`{key}` removed a file outside the config");
    }

    Ok(())
}

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
        let run = sandbox.config(&args)?;
        assert_eq!(run.code, EX_DATAERR as i32, "should reject {args:?}");
    }

    assert_eq!(std::fs::read_to_string(&escape)?, "keep me");

    Ok(())
}

#[test]
fn secret_values_are_shown_only_when_read_by_name() -> Result {
    let sandbox = Sandbox::new()?;
    sandbox.config(&["set", "demo", "api-key=s3cret-value"])?;

    let shown = sandbox.config(&["show", "demo"])?;
    assert!(
        !shown.stdout.contains("s3cret-value"),
        "`show` disclosed a secret: {}",
        shown.stdout
    );
    assert!(shown.stdout.contains("api-key"), "`show` omitted the name");

    let got = sandbox.config(&["get", "demo", "api-key"])?;
    assert_eq!(got.stdout.trim(), "s3cret-value");

    Ok(())
}

#[test]
fn a_rejected_batch_changes_nothing() -> Result {
    let sandbox = Sandbox::new()?;
    sandbox.config(&["set", "demo", "host=first"])?;

    let run = sandbox.config(&["set", "demo", "host=second", "nonexistent=value"])?;
    assert_eq!(run.code, EX_USAGE as i32);

    let host = sandbox.config(&["get", "demo", "host", "--stored"])?;
    assert_eq!(host.stdout.trim(), "first", "a rejected batch was applied");

    Ok(())
}

#[cfg(unix)]
#[test]
fn stored_values_are_private_to_the_user() -> Result {
    use std::os::unix::fs::PermissionsExt;

    let sandbox = Sandbox::new()?;
    sandbox.config(&["set", "demo", "api-key=s3cret-value"])?;

    let mode =
        |path: &Path| -> Result<u32> { Ok(std::fs::metadata(path)?.permissions().mode() & 0o777) };

    assert_eq!(mode(&sandbox.value_file("api-key"))?, 0o600);
    assert_eq!(mode(&sandbox.root().join("configs/default/demo"))?, 0o700);

    Ok(())
}

#[test]
fn get_resolves_the_environment_then_the_stored_value_then_the_default() -> Result {
    let sandbox = Sandbox::new()?;

    let run = sandbox.config(&["get", "demo", "host"])?;
    assert_eq!(run.stdout.trim(), "default.example");

    sandbox.config(&["set", "demo", "host=stored.example"])?;
    let run = sandbox.config(&["get", "demo", "host"])?;
    assert_eq!(run.stdout.trim(), "stored.example");

    let env = [(HOST_ENV, "env.example")];
    let run = sandbox.config_env(&["get", "demo", "host"], &env)?;
    assert_eq!(run.stdout.trim(), "env.example");

    // `--stored` answers a different question, and ignores the environment
    let run = sandbox.config_env(&["get", "demo", "host", "--stored"], &env)?;
    assert_eq!(run.stdout.trim(), "stored.example");

    Ok(())
}

#[test]
fn inspect_reports_unmet_configuration_through_its_exit_status() -> Result {
    let sandbox = Sandbox::new()?;

    let run = sandbox.module(&["inspect", "demo"])?;
    assert_eq!(
        run.code, EX_CONFIG as i32,
        "`api-key` is required and unset"
    );

    // `host` is unset too, but its default satisfies it
    assert!(run.stdout.contains("host"));

    sandbox.config(&["set", "demo", "api-key=s3cret-value"])?;
    let run = sandbox.module(&["inspect", "demo"])?;
    assert_eq!(run.code, EX_OK as i32);

    // a value from the environment counts just as much as a stored one
    sandbox.config(&["unset", "demo", "api-key"])?;
    let run = sandbox.module_env(&["inspect", "demo"], &[(KEY_ENV, "from-env")])?;
    assert_eq!(run.code, EX_OK as i32);

    Ok(())
}

#[test]
fn setup_without_a_terminal_fails_rather_than_waiting() -> Result {
    let sandbox = Sandbox::new()?;

    let run = sandbox.config(&["setup", "demo"])?;
    assert_eq!(run.code, EX_UNAVAILABLE as i32);

    Ok(())
}
