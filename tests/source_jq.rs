// This is free and unencumbered software released into the public domain.

#![cfg(all(unix, feature = "source"))]

extern crate alloc;

use alloc::{format, string::String};
use clientele::SysexitsError::*;
use std::{
    os::unix::fs::{PermissionsExt, symlink},
    process::{Command, Output, Stdio},
};
use temp_dir::TempDir;

type Result<T = ()> = core::result::Result<T, Box<dyn core::error::Error>>;

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
        let mut process = Command::new(env!("CARGO_BIN_EXE_asimov"));
        process
            .args(["source", command])
            .args(options)
            .arg("https://jq.example/data")
            .env("ASIMOV_ROOT", self.0.path())
            .env("HOME", self.0.path())
            .env("ASIMOV_KEYRING_BACKEND", "file")
            .env("DO_NOT_TRACK", "1")
            .env_remove("TYPESAFE_API_TOKEN")
            .env("PATH", self.0.child("bin"))
            .current_dir(self.0.path())
            .stdin(Stdio::null());
        process
    }
}

#[test]
fn source_commands_require_jev_credentials_and_validate_input_before_http() -> Result {
    let sandbox = Sandbox::with_script("#!/bin/sh\nprintf '%s\\n' 'not JSON'\n")?;
    for command in ["fetch", "list"] {
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
fn source_commands_report_subprocess_failures_after_output() -> Result {
    let sandbox = Sandbox::with_script("#!/bin/sh\nprintf '%s\\n' '[1,2]'\nexit 42\n")?;
    for command in ["fetch", "list"] {
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
fn source_commands_preserve_module_sysexits() -> Result {
    for code in [EX_NOINPUT, EX_NOPERM, EX_TEMPFAIL] {
        for payload in ["", "[1,2]\n"] {
            let sandbox = Sandbox::with_script(&format!(
                "#!/bin/sh\nprintf '%s' '{payload}'\nexit {}\n",
                code as i32
            ))?;
            for command in ["fetch", "list"] {
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
fn source_commands_preserve_the_first_failure_in_url_order() -> Result {
    let sandbox = Sandbox::with_script(
        "#!/bin/sh\nfor url do :; done\ncase \"$url\" in\n\
         */missing) exit 66;;\n*/unavailable) exit 69;;\n\
         *) printf '%s\\n' '[1,2]';;\nesac\n",
    )?;
    for command in ["fetch", "list"] {
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
fn source_commands_preserve_all_jq_results() -> Result {
    let sandbox = Sandbox::new()?;
    for command in ["fetch", "list"] {
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
fn source_commands_accept_empty_jq_output() -> Result {
    let sandbox = Sandbox::new()?;
    for command in ["fetch", "list"] {
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
fn source_commands_report_jq_errors_after_a_result() -> Result {
    let sandbox = Sandbox::new()?;
    for command in ["fetch", "list"] {
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
