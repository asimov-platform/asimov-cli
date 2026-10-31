// This is free and unencumbered software released into the public domain.

use clientele::SysexitsError::*;
use std::process::{Command, Stdio};
use temp_dir::TempDir;

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

const MANIFEST: &str = r#"{
  "name": "demo",
  "config": { "variables": [{ "name": "host", "default_value": "example" }] }
}"#;

/// Runs `asimov module <args>` with stdout connected to a closed pipe, as
/// when piping into a reader that exits early (e.g., `| head -1`).
fn run_with_closed_stdout(root: &TempDir, args: &[&str]) -> Result<(i32, String)> {
    let (reader, writer) = std::io::pipe()?;
    drop(reader);
    let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
        .arg("module")
        .args(args)
        .current_dir(root.path())
        .env("HOME", root.path())
        .env("ASIMOV_ROOT", root.path())
        .env("ASIMOV_KEYRING_BACKEND", "file")
        .env("DO_NOT_TRACK", "1")
        .stdin(Stdio::null())
        .stdout(writer)
        .stderr(Stdio::piped())
        .output()?;
    Ok((
        output.status.code().expect("should exit normally"),
        String::from_utf8(output.stderr)?,
    ))
}

#[test]
fn closed_stdout_stops_output_quietly() -> Result {
    let root = TempDir::new()?;
    let module_dir = root.child("modules/installed/demo");
    std::fs::create_dir_all(module_dir.join("doc"))?;
    std::fs::write(module_dir.join("manifest.json"), MANIFEST)?;
    std::fs::write(module_dir.join("doc/README.md"), "# Demo\n")?;

    for args in [
        &["list"][..],
        &["list", "--output=jsonl"],
        &["config", "get", "demo", "host"],
        &["config", "show", "demo"],
        &["config", "show", "demo", "--output=json"],
        &["doc", "demo"],
        &["inspect", "demo"],
        &["inspect", "demo", "--output=json"],
    ] {
        let (code, stderr) = run_with_closed_stdout(&root, args)?;
        assert_eq!(code, EX_OK as i32, "{args:?}: {stderr}");
        assert!(stderr.is_empty(), "{args:?}: {stderr}");
    }
    Ok(())
}
