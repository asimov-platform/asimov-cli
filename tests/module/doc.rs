// This is free and unencumbered software released into the public domain.

use clientele::SysexitsError::*;
use std::process::{Command, Stdio};
use temp_dir::TempDir;

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn doc(root: &TempDir) -> Result<Run> {
    let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
        .args(["module", "doc", "demo"])
        .env("ASIMOV_ROOT", root.path())
        .stdin(Stdio::null())
        .output()?;

    Ok(Run {
        code: output.status.code().expect("should exit normally"),
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
    })
}

#[test]
fn doc_prints_the_installed_readme() -> Result {
    let root = TempDir::new()?;
    let doc_dir = root.child("modules/installed/demo/doc");
    std::fs::create_dir_all(&doc_dir)?;
    std::fs::write(doc_dir.join("README.md"), "# Demo\n\nHow to use it.\n")?;

    let run = doc(&root)?;

    assert_eq!(run.code, EX_OK as i32);
    assert_eq!(run.stdout, "# Demo\n\nHow to use it.\n");

    Ok(())
}

#[test]
fn an_installed_module_without_a_readme_is_not_reported_as_missing() -> Result {
    let root = TempDir::new()?;
    let module_dir = root.child("modules/installed/demo");
    std::fs::create_dir_all(&module_dir)?;
    std::fs::write(module_dir.join("manifest.json"), r#"{ "name": "demo" }"#)?;

    let run = doc(&root)?;

    assert_eq!(run.code, EX_NOINPUT as i32);
    assert!(
        run.stdout.is_empty(),
        "should not print anything: {}",
        run.stdout
    );
    assert!(
        !run.stderr.contains("not installed"),
        "should not claim the module is missing: {}",
        run.stderr
    );

    Ok(())
}

#[test]
fn doc_fails_when_the_module_is_not_installed() -> Result {
    let root = TempDir::new()?;

    let run = doc(&root)?;

    assert_eq!(run.code, EX_UNAVAILABLE as i32);
    assert!(
        run.stdout.is_empty(),
        "should not print anything: {}",
        run.stdout
    );
    assert!(
        run.stderr.contains("not installed"),
        "should say the module is missing: {}",
        run.stderr
    );

    Ok(())
}
