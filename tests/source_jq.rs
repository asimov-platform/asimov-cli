// This is free and unencumbered software released into the public domain.

#![cfg(all(unix, feature = "source"))]

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
        Ok(Command::new(env!("CARGO_BIN_EXE_asimov"))
            .args(["source", command])
            .args(options)
            .arg("https://jq.example/data")
            .env("ASIMOV_ROOT", self.0.path())
            .env("PATH", self.0.child("bin"))
            .current_dir(self.0.path())
            .stdin(Stdio::null())
            .output()?)
    }
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
