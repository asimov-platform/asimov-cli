// This is free and unencumbered software released into the public domain.

#![cfg(all(unix, feature = "resource"))]

extern crate alloc;

use alloc::{format, string::String};
use clientele::SysexitsError::*;
use serde_json::{Value, json};
use std::{
    os::unix::fs::{PermissionsExt, symlink},
    process::{Command, Output, Stdio},
};
use temp_dir::TempDir;

type Result<T = ()> = core::result::Result<T, Box<dyn core::error::Error>>;

const OPERATIONS: &[&str] = &["fetch", "list"];

struct Sandbox(TempDir);

impl Sandbox {
    fn new() -> Result<Self> {
        Self::with_script("#!/bin/sh\nprintf '%s\\n' '[1,2]' '[]' '[3,4]'\n")
    }

    fn with_script(script: &str) -> Result<Self> {
        let root = TempDir::new()?;
        let installed = root.child("modules/installed/jq-test");
        std::fs::create_dir_all(&installed)?;
        std::fs::write(
            installed.join("manifest.json"),
            r#"{
                "name": "jq-test",
                "provides": {
                    "programs": ["asimov-jq-test-fetcher", "asimov-jq-test-lister"]
                },
                "handles": {"url_prefixes": ["https://jq.example/"]}
            }"#,
        )?;
        std::fs::create_dir_all(root.child("modules/enabled"))?;
        symlink(&installed, root.child("modules/enabled/jq-test"))?;
        std::fs::create_dir_all(root.child("bin"))?;
        for program in ["fetcher", "lister"] {
            let path = root.child(format!("bin/asimov-jq-test-{program}"));
            std::fs::write(&path, script)?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        }
        Ok(Self(root))
    }

    fn run(&self, command: &str, expression: &str) -> Result<Output> {
        self.run_options(command, &["--jq", expression])
    }

    fn run_options(&self, command: &str, options: &[&str]) -> Result<Output> {
        Ok(self.command(command, options).output()?)
    }

    fn command(&self, command: &str, options: &[&str]) -> Command {
        self.command_path(&["resource", command], options)
    }

    fn command_path(&self, path: &[&str], options: &[&str]) -> Command {
        let mut process = Command::new(env!("CARGO_BIN_EXE_asimov"));
        process
            .args(path)
            .args(options)
            .arg("https://jq.example/data")
            .env("ASIMOV_ROOT", self.0.path())
            .env("HOME", self.0.path())
            .env("ASIMOV_KEYRING_BACKEND", "file")
            .env("DO_NOT_TRACK", "1")
            .env_remove("TYPESAFE_API_TOKEN")
            .env_remove("ASIMOV_RELAY_URL")
            .env("PATH", self.0.child("bin"))
            .current_dir(self.0.path())
            .stdin(Stdio::null());
        process
    }
}

#[test]
fn resource_commands_require_jev_credentials_and_validate_input_before_http() -> Result {
    let sandbox = Sandbox::with_script("#!/bin/sh\nprintf '%s\\n' 'not JSON'\n")?;
    for &command in OPERATIONS {
        let output = sandbox.run_options(command, &["--jev", "The name is Ukrainian"])?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(EX_CONFIG as i32),
            "{command}: {stderr}"
        );
        assert!(
            stderr.contains("--jev requires TYPESAFE_API_TOKEN"),
            "{stderr}"
        );
        assert!(output.stdout.is_empty());

        let output = sandbox
            .command(command, &["--jev", "The name is Ukrainian"])
            .env("TYPESAFE_API_TOKEN", "fixture-token")
            .output()?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(EX_DATAERR as i32),
            "{command}: {stderr}"
        );
        assert!(stderr.contains("invalid JSON in Jev input"), "{stderr}");
        assert!(output.stdout.is_empty());
    }
    Ok(())
}

#[test]
fn list_forwards_cursor_bounds_and_explicit_offsets_to_the_module() -> Result {
    let sandbox = Sandbox::with_script("#!/bin/sh\nprintf '%s\\n' \"$@\"\n")?;
    for (options, expected) in [
        (vec!["--after=urn:entry:a"], "--after=urn:entry:a\n"),
        (
            vec!["--before=HTTPS://Example.COM/a%2fb?x=a=b#end"],
            "--before=HTTPS://Example.COM/a%2fb?x=a=b#end\n",
        ),
        (
            vec![
                "--sort=-name",
                "--before=urn:entry:z",
                "--after=urn:entry:a",
                "--limit=10",
            ],
            "--sort=-name\n--before=urn:entry:z\n--after=urn:entry:a\n--limit=10\n",
        ),
        (vec!["--offset=0", "--limit=10"], "--offset=0\n--limit=10\n"),
        (vec![], ""),
    ] {
        let output = sandbox.run_options("list", &options)?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{options:?}: {stderr}");
        assert_eq!(
            output.stdout,
            format!("{expected}https://jq.example/data\n").as_bytes()
        );
    }
    Ok(())
}

#[test]
fn resource_commands_report_subprocess_failures_after_output() -> Result {
    let sandbox = Sandbox::with_script("#!/bin/sh\nprintf '%s\\n' '[1,2]'\nexit 42\n")?;
    for &command in OPERATIONS {
        for (options, expected) in [(vec![], "[1,2]\n"), (vec!["--jq", ".[]"], "1\n2\n")] {
            let output = sandbox.run_options(command, &options)?;
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert_eq!(
                output.status.code(),
                Some(EX_UNAVAILABLE as i32),
                "{command}: {stderr}"
            );
            assert_eq!(output.stdout, expected.as_bytes(), "{command}: {stderr}");
            assert!(stderr.contains("execution failed"), "{command}: {stderr}");
        }
    }
    Ok(())
}

#[test]
fn resource_commands_preserve_module_sysexits() -> Result {
    for code in [EX_NOINPUT, EX_NOPERM, EX_TEMPFAIL] {
        for payload in ["", "[1,2]\n"] {
            let sandbox = Sandbox::with_script(&format!(
                "#!/bin/sh\nprintf '%s' '{payload}'\nexit {}\n",
                code as i32
            ))?;
            for &command in OPERATIONS {
                let output = sandbox.run_options(command, &[])?;
                assert_eq!(
                    output.status.code(),
                    Some(code as i32),
                    "{command}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(output.stdout, payload.as_bytes());
            }
        }
    }
    Ok(())
}

#[test]
fn resource_commands_preserve_the_first_failure_in_url_order() -> Result {
    let sandbox = Sandbox::with_script(
        "#!/bin/sh\nfor url do :; done\ncase \"$url\" in\n\
         */missing) exit 66;;\n*/unavailable) exit 69;;\n\
         *) printf '%s\\n' '[1,2]';;\nesac\n",
    )?;
    for &command in OPERATIONS {
        for (paths, code) in [
            (["missing", "unavailable"], EX_NOINPUT),
            (["unavailable", "missing"], EX_UNAVAILABLE),
        ] {
            let urls = paths.map(|path| format!("https://jq.example/{path}"));
            let output = sandbox
                .command(command, &[])
                .args(urls)
                .arg("https://jq.example/success")
                .output()?;
            assert_eq!(
                output.status.code(),
                Some(code as i32),
                "{command}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            // Failure must not prevent subsequent URLs from being processed.
            assert_eq!(output.stdout, b"[1,2]\n[1,2]\n");
        }
    }
    Ok(())
}

#[test]
fn resource_commands_preserve_all_jq_results() -> Result {
    let sandbox = Sandbox::new()?;
    for &command in OPERATIONS {
        let output = sandbox.run(command, ".[]")?;
        assert_eq!(
            output.status.code(),
            Some(EX_OK as i32),
            "{command}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"1\n2\n3\n4\n", "{command}");
    }
    Ok(())
}

#[test]
fn resource_commands_accept_empty_jq_output() -> Result {
    let sandbox = Sandbox::new()?;
    for &command in OPERATIONS {
        let output = sandbox.run(command, "empty")?;
        assert_eq!(
            output.status.code(),
            Some(EX_OK as i32),
            "{command}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty(), "{command}");
    }
    Ok(())
}

#[test]
fn resource_commands_report_jq_errors_after_a_result() -> Result {
    let sandbox = Sandbox::new()?;
    for &command in OPERATIONS {
        let output = sandbox.run(command, r#".[], error("late failure")"#)?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(EX_DATAERR as i32),
            "{command}: {stderr}"
        );
        assert!(
            stderr.contains("jq filtering failed"),
            "{command}: {stderr}"
        );
        assert!(stderr.contains("late failure"), "{command}: {stderr}");
    }
    Ok(())
}

#[test]
fn json_selection_preserves_literal_properties_nested_values_and_array_shape() -> Result {
    let record = json!({
        "@context": {"name": "https://schema.org/name"},
        "@id": "urn:person:1", "@type": "Person", "name": "Ada", "url": null,
        "author": {"name": "Grace", "extra": true},
        "a.b": 1, "a\"b": 2, "schema:name": "Ada", "naïve": true, "extra": 3,
    });
    let selected = json!({
        "@id": "urn:person:1", "name": "Ada", "url": null,
        "author": {"name": "Grace", "extra": true},
        "a.b": 1, "a\"b": 2, "schema:name": "Ada", "naïve": true,
    });
    let sandbox = Sandbox::with_script(&format!(
        "#!/bin/sh\nprintf '%s\\r\\n' '{record}' '[{record},{{\"extra\":true}}]'\nprintf '%s' '[]'\n"
    ))?;
    for &command in OPERATIONS {
        for path in [
            vec!["resource", command],
            vec!["source", command],
            vec![command],
        ] {
            let output = sandbox
                .command_path(
                    &path,
                    &[
                        "--output=jsonl",
                        "--json=@id,name,url,missing",
                        "--json",
                        "author,a.b,a\"b,schema:name,naïve,name",
                    ],
                )
                .output()?;
            assert!(output.status.success(), "{path:?}: {output:?}");
            let values: Vec<Value> = serde_json::Deserializer::from_slice(&output.stdout)
                .into_iter()
                .collect::<core::result::Result<_, _>>()?;
            assert_eq!(values, [selected.clone(), json!([selected, {}]), json!([])]);
            assert!(output.stdout.ends_with(b"\n"));
        }
        // JSON-LD metadata is retained only when explicitly requested.
        let output = sandbox.run_options(command, &["--json=@context,@type"])?;
        assert!(output.status.success(), "{output:?}");
        let values: Vec<Value> = serde_json::Deserializer::from_slice(&output.stdout)
            .into_iter()
            .collect::<core::result::Result<_, _>>()?;
        assert_eq!(
            values[0],
            json!({"@context": record["@context"], "@type": "Person"})
        );
    }
    Ok(())
}

#[test]
fn json_selection_precedes_jq_and_preserves_all_or_no_results() -> Result {
    let sandbox =
        Sandbox::with_script("#!/bin/sh\nprintf '%s\\n' '{\"name\":\"Ada\",\"extra\":true}'\n")?;
    for &command in OPERATIONS {
        for (expression, expected) in [
            (
                r#"if has("extra") then error("unselected property") else {label: .name}, .name end"#,
                "{\"label\":\"Ada\"}\n\"Ada\"\n",
            ),
            ("empty", ""),
        ] {
            let output = sandbox.run_options(command, &["--json=name", "--jq", expression])?;
            assert!(output.status.success(), "{command}: {output:?}");
            assert_eq!(output.stdout, expected.as_bytes());
        }
        let output = sandbox.run_options(
            command,
            &["--json=name", "--jq", r#".name, error("late failure")"#],
        )?;
        assert_eq!(output.status.code(), Some(EX_DATAERR as i32), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("jq filtering failed"));
        assert!(output.stdout.is_empty());
    }
    Ok(())
}

#[test]
fn json_selection_reports_invalid_records_after_prior_output() -> Result {
    for record in [
        "not JSON",
        "42",
        "true",
        "null",
        r#""text""#,
        "[{},42]",
        "[[{}]]",
    ] {
        let sandbox = Sandbox::with_script(&format!(
            "#!/bin/sh\nprintf '%s\\n' '{{\"name\":\"first\",\"extra\":1}}' '{record}'\n"
        ))?;
        for &command in OPERATIONS {
            let output = sandbox.run_options(command, &["--json=name"])?;
            assert_eq!(output.status.code(), Some(EX_DATAERR as i32), "{output:?}");
            assert_eq!(output.stdout, b"{\"name\":\"first\"}\n");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("JSON property selection failed"),
                "{stderr}"
            );
            assert!(stderr.contains("https://jq.example/data"), "{stderr}");
        }
    }
    Ok(())
}

#[test]
fn json_selection_rejects_incompatible_formats_before_module_execution() -> Result {
    let sandbox = Sandbox::with_script("#!/bin/sh\nexit 99\n")?;
    for &command in OPERATIONS {
        for format in ["url", "csv", "turtle", "json"] {
            let output = sandbox.run_options(command, &["--json=name", "--output", format])?;
            assert_eq!(output.status.code(), Some(EX_USAGE as i32), "{output:?}");
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("--json requires JSONL"));
        }
    }
    Ok(())
}

#[test]
fn json_selection_preserves_terminal_subprocess_failures() -> Result {
    let sandbox = Sandbox::with_script(
        "#!/bin/sh\nprintf '%s\\n' '{\"name\":\"Ada\",\"extra\":true}'\nexit 66\n",
    )?;
    for &command in OPERATIONS {
        let output = sandbox.run_options(command, &["--json=name"])?;
        assert_eq!(output.status.code(), Some(EX_NOINPUT as i32), "{output:?}");
        assert_eq!(output.stdout, b"{\"name\":\"Ada\"}\n");
    }
    Ok(())
}
