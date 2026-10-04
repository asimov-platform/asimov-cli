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
    stderr: String,
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
        self.module_env_input(args, env, None)
    }

    fn module_env_input(
        &self,
        args: &[&str],
        env: &[(&str, &str)],
        input: Option<&[u8]>,
    ) -> Result<Run> {
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

        let output = if let Some(input) = input {
            use std::io::Write;
            let mut child = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            child.stdin.take().expect("piped stdin").write_all(input)?;
            child.wait_with_output()?
        } else {
            command.output()?
        };
        Ok(Run {
            code: output.status.code().expect("should exit normally"),
            stdout: String::from_utf8(output.stdout)?,
            stderr: String::from_utf8(output.stderr)?,
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
fn configuration_readers_reject_unsafe_and_colliding_names() -> Result {
    for names in [
        vec!["../escape"],
        vec!["CON"],
        vec!["nul.txt"],
        vec!["com1"],
        vec!["LPT9.txt"],
        vec!["host."],
        vec!["host", "host"],
        vec!["host", "HOST"],
    ] {
        let manifest = serde_json::json!({"name": "demo", "config": {"variables":
            names.iter().map(|name| serde_json::json!({"name": name, "default_value": "value"})).collect::<Vec<_>>()
        }});
        let sandbox = Sandbox::with_manifest(&manifest.to_string())?;
        for args in [
            vec!["config", "show", "demo"],
            vec!["inspect", "demo"],
            vec!["install", "demo"],
        ] {
            let run = sandbox.module(&args)?;
            assert_eq!(run.code, EX_DATAERR as i32, "{names:?}: {args:?}");
        }
    }
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

#[cfg(unix)]
#[test]
fn partial_config_batches_leave_new_values_private() -> Result {
    use std::os::unix::fs::PermissionsExt;
    let sandbox = Sandbox::new()?;
    std::fs::create_dir_all(sandbox.value_file("host"))?;
    let run = sandbox.config(&["set", "demo", "api-key=first-secret", "host=second-value"])?;
    assert_eq!(run.code, EX_IOERR as i32, "{}", run.stderr);
    assert!(run.stderr.contains("asimov:"));
    assert!(!run.stderr.contains("first-secret"));
    assert_eq!(
        std::fs::read_to_string(sandbox.value_file("api-key"))?,
        "first-secret"
    );
    assert_eq!(
        std::fs::metadata(sandbox.value_file("api-key"))?
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert!(sandbox.value_file("host").is_dir());
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_failed_write_still_repairs_existing_configuration_permissions() -> Result {
    use std::os::unix::fs::PermissionsExt;

    let sandbox = Sandbox::new()?;
    let demo_dir = sandbox.root().join("configs/default/demo");
    let old_value = demo_dir.join("api-key");
    let invalid_value = demo_dir.join("host");
    std::fs::create_dir_all(&invalid_value)?;
    std::fs::write(&old_value, "old-secret")?;
    std::fs::set_permissions(&demo_dir, std::fs::Permissions::from_mode(0o755))?;
    std::fs::set_permissions(&old_value, std::fs::Permissions::from_mode(0o644))?;

    let run = sandbox.config(&["set", "demo", "host=example.test"])?;
    assert_ne!(run.code, EX_OK as i32);

    let mode =
        |path: &Path| -> Result<u32> { Ok(std::fs::metadata(path)?.permissions().mode() & 0o777) };
    assert_eq!(mode(&demo_dir)?, 0o700);
    assert_eq!(mode(&old_value)?, 0o600);

    Ok(())
}

#[cfg(unix)]
#[test]
fn setting_a_value_repairs_only_that_modules_configuration_permissions() -> Result {
    use std::os::unix::fs::PermissionsExt;

    let sandbox = Sandbox::new()?;
    let profile_dir = sandbox.root().join("configs/default");
    let demo_dir = profile_dir.join("demo");
    let nested_dir = demo_dir.join("nested");
    let other_dir = profile_dir.join("other");
    std::fs::create_dir_all(&nested_dir)?;
    std::fs::create_dir_all(&other_dir)?;

    let old_value = demo_dir.join("api-key");
    let nested_value = nested_dir.join("old");
    let other_value = other_dir.join("key");
    std::fs::write(&old_value, "old-secret")?;
    std::fs::write(&nested_value, "old-secret")?;
    std::fs::write(&other_value, "other-secret")?;

    for dir in [&profile_dir, &demo_dir, &nested_dir, &other_dir] {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755))?;
    }
    for file in [&old_value, &nested_value, &other_value] {
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o644))?;
    }

    sandbox.config(&["set", "demo", "host=example.test"])?;

    let mode = |path: &Path| -> Result<u32> {
        Ok(std::fs::symlink_metadata(path)?.permissions().mode() & 0o777)
    };
    assert_eq!(mode(&demo_dir)?, 0o700);
    assert_eq!(mode(&nested_dir)?, 0o700);
    assert_eq!(mode(&old_value)?, 0o600);
    assert_eq!(mode(&nested_value)?, 0o600);
    assert_eq!(mode(&sandbox.value_file("host"))?, 0o600);
    assert_eq!(mode(&profile_dir)?, 0o755);
    assert_eq!(mode(&other_dir)?, 0o755);
    assert_eq!(mode(&other_value)?, 0o644);

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
fn inspect_redacts_secret_defaults_in_every_output_format() -> Result {
    let mut manifest: serde_json::Value = serde_json::from_str(MANIFEST)?;
    manifest["config"]["variables"][0]["default_value"] = "secret-default-value".into();
    let sandbox = Sandbox::with_manifest(&serde_json::to_string(&manifest)?)?;

    for format in ["cli", "json"] {
        let run = sandbox.module(&["inspect", "demo", "--output", format])?;
        assert_eq!(run.code, EX_OK as i32);
        assert!(
            !run.stdout.contains("secret-default-value"),
            "{format} inspection disclosed a secret: {}",
            run.stdout
        );
        assert!(run.stdout.contains("default.example"));
        assert!(run.stdout.contains("api-key"));

        if format == "json" {
            let report: serde_json::Value = serde_json::from_str(&run.stdout)?;
            assert!(report["config"][0]["default"].is_null());
            assert_eq!(report["config"][0]["set"], true);
            assert_eq!(report["config"][0]["required"], false);
            assert!(report["manifest"].is_object());
        }
    }

    let stored: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        sandbox.root().join("modules/installed/demo/manifest.json"),
    )?)?;
    assert_eq!(stored, manifest, "inspection must not modify the manifest");

    Ok(())
}

#[test]
fn module_list_emits_valid_jsonl_for_special_characters() -> Result {
    let mut manifest: serde_json::Value = serde_json::from_str(MANIFEST)?;
    let label = "quoted \"label\"\nwith\\slashes\r\t";
    manifest["label"] = label.into();
    let sandbox = Sandbox::with_manifest(&serde_json::to_string(&manifest)?)?;
    let run = sandbox.module(&["list", "--output", "jsonl"])?;
    assert_eq!(run.code, EX_OK as i32);
    let lines: Vec<_> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 1);
    let record: serde_json::Value = serde_json::from_str(lines[0])?;
    assert_eq!(record["label"], label);
    assert_eq!(record["name"], "demo");
    assert_eq!(record["@type"], "AsimovModule");
    assert_eq!(record["@id"], "https://asimov.directory/modules/demo");
    assert_eq!(record["version"], "");
    assert_eq!(record["enabled"], false);
    Ok(())
}

#[test]
fn inspect_rejects_unreadable_configuration_before_emitting_a_report() -> Result {
    for directory in [false, true] {
        let sandbox = Sandbox::new()?;
        std::fs::create_dir_all(sandbox.root().join("configs/default/demo"))?;
        if directory {
            std::fs::create_dir(sandbox.value_file("host"))?;
        } else {
            std::fs::write(sandbox.value_file("host"), b"\xff")?;
        }
        for format in ["cli", "json"] {
            let run = sandbox.module(&["inspect", "demo", "--output", format])?;
            assert_eq!(run.code, EX_IOERR as i32, "{}", run.stderr);
            assert!(run.stdout.is_empty());
            assert!(run.stderr.contains("host"));
        }
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn config_readers_report_permission_failures_even_with_defaults() -> Result {
    use std::os::unix::fs::PermissionsExt;
    let sandbox = Sandbox::new()?;
    std::fs::create_dir_all(sandbox.root().join("configs/default/demo"))?;
    let path = sandbox.value_file("host");
    std::fs::write(&path, "private-value")?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))?;
    // Privileged test runners can still read mode-000 files.
    if std::fs::read_to_string(&path).is_err() {
        for args in [vec!["inspect", "demo"], vec!["config", "show", "demo"]] {
            let run = sandbox.module(&args)?;
            assert_eq!(run.code, EX_IOERR as i32, "{}", run.stderr);
            assert!(run.stdout.is_empty());
            assert!(!run.stderr.contains("private-value"));
        }
    }
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[test]
fn config_show_reports_read_errors_instead_of_unset_values() -> Result {
    for directory in [false, true] {
        let sandbox = Sandbox::new()?;
        std::fs::create_dir_all(sandbox.root().join("configs/default/demo"))?;
        if directory {
            std::fs::create_dir(sandbox.value_file("host"))?;
        } else {
            std::fs::write(sandbox.value_file("host"), b"\xff")?;
        }
        for format in ["cli", "json"] {
            let run = sandbox.config(&["show", "demo", "--output", format])?;
            assert_eq!(run.code, EX_IOERR as i32, "{}", run.stderr);
            assert!(run.stdout.is_empty());
            assert!(run.stderr.contains("host"));
        }
        let run = sandbox.config_env(
            &["show", "demo", "--output", "json"],
            &[(HOST_ENV, "override")],
        )?;
        assert_eq!(run.code, EX_OK as i32, "{}", run.stderr);
        let rows: serde_json::Value = serde_json::from_str(&run.stdout)?;
        assert_eq!(rows[1]["source"], "environment");
        assert_eq!(rows[1]["value"], "override");
    }
    Ok(())
}

#[test]
fn config_show_keeps_provenance_and_values_consistent() -> Result {
    let sandbox = Sandbox::new()?;
    let run = sandbox.config(&["show", "demo", "--output", "json"])?;
    assert_eq!(run.code, EX_OK as i32);
    let rows: serde_json::Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(rows[0]["source"], "unset");
    assert!(rows[0]["value"].is_null());
    assert_eq!(rows[1]["source"], "default");
    assert_eq!(rows[1]["value"], "default.example");
    assert_eq!(
        sandbox
            .config(&["set", "demo", "host=  stored  ", "api-key=private-secret"])?
            .code,
        EX_OK as i32
    );
    let run = sandbox.config(&["show", "demo", "--output", "json"])?;
    assert_eq!(run.code, EX_OK as i32);
    assert!(!run.stdout.contains("private-secret"));
    let rows: serde_json::Value = serde_json::from_str(&run.stdout)?;
    assert_eq!(rows[0]["source"], "stored");
    assert!(rows[0]["value"].is_null());
    assert_eq!(rows[1]["source"], "stored");
    assert_eq!(rows[1]["value"], "  stored  ");
    Ok(())
}

#[test]
fn effective_config_retrieval_preserves_whitespace_and_precedence() -> Result {
    for value in ["  spaced  ", "one\ntwo\n", "one\r\ntwo\r\n", ""] {
        let mut manifest: serde_json::Value = serde_json::from_str(MANIFEST)?;
        manifest["config"]["variables"][1]["default_value"] = value.into();
        let sandbox = Sandbox::with_manifest(&manifest.to_string())?;
        let default = sandbox.config(&["get", "demo", "host"])?;
        assert_eq!(default.code, EX_OK as i32);
        assert_eq!(default.stdout, format!("{value}\n"));
        let stored = format!("stored {value} ");
        let assignment = format!("host={stored}");
        assert_eq!(
            sandbox.config(&["set", "demo", &assignment])?.code,
            EX_OK as i32
        );
        let run = sandbox.config(&["get", "demo", "host"])?;
        assert_eq!(run.code, EX_OK as i32);
        assert_eq!(run.stdout, format!("{stored}\n"));
        let run = sandbox.config_env(&["get", "demo", "host"], &[(HOST_ENV, value)])?;
        assert_eq!(run.code, EX_OK as i32);
        assert_eq!(run.stdout, format!("{value}\n"));
    }
    Ok(())
}

#[test]
fn stored_config_retrieval_preserves_value_whitespace() -> Result {
    let sandbox = Sandbox::new()?;
    for value in [
        "  padded  ",
        "\t",
        "first\nsecond\n",
        "first\r\nsecond\r\n",
        "",
    ] {
        let assignment = format!("host={value}");
        let run = sandbox.config(&["set", "demo", &assignment])?;
        assert_eq!(run.code, EX_OK as i32);
        let run = sandbox.config(&["get", "demo", "host", "--stored"])?;
        assert_eq!(run.code, EX_OK as i32);
        assert_eq!(run.stdout, format!("{value}\n"));

        let json = serde_json::to_vec(&serde_json::json!({"host": value}))?;
        let run = sandbox.module_env_input(
            &["config", "set", "demo", "--from-json"],
            &[],
            Some(&json),
        )?;
        assert_eq!(run.code, EX_OK as i32);
        let run = sandbox.config(&["get", "demo", "host", "--stored"])?;
        assert_eq!(run.code, EX_OK as i32);
        assert_eq!(run.stdout, format!("{value}\n"));

        // --stdin removes one transport newline, not whitespace in the value.
        let piped = format!("{value}\r\n");
        let run = sandbox.module_env_input(
            &["config", "set", "demo", "host", "--stdin"],
            &[],
            Some(piped.as_bytes()),
        )?;
        assert_eq!(run.code, EX_OK as i32);
        let run = sandbox.config(&["get", "demo", "host", "--stored"])?;
        assert_eq!(run.code, EX_OK as i32);
        assert_eq!(run.stdout, format!("{value}\n"));
    }
    Ok(())
}

#[test]
fn setup_without_a_terminal_fails_rather_than_waiting() -> Result {
    let sandbox = Sandbox::new()?;

    let run = sandbox.config(&["setup", "demo"])?;
    assert_eq!(run.code, EX_UNAVAILABLE as i32);

    Ok(())
}
