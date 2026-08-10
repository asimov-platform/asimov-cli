// This is free and unencumbered software released into the public domain.

use clientele::SysexitsError::*;
use std::process::{Command, Stdio};
use temp_dir::TempDir;

type Result<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

fn doc(root: &TempDir) -> Result<(i32, String)> {
    let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
        .args(["module", "doc", "demo"])
        .env("ASIMOV_ROOT", root.path())
        .stdin(Stdio::null())
        .output()?;

    Ok((
        output.status.code().expect("should exit normally"),
        String::from_utf8(output.stdout)?,
    ))
}

#[test]
fn doc_prints_the_installed_readme() -> Result {
    let root = TempDir::new()?;
    let doc_dir = root.child("modules/installed/demo/doc");
    std::fs::create_dir_all(&doc_dir)?;
    std::fs::write(doc_dir.join("README.md"), "# Demo\n\nHow to use it.\n")?;

    let (code, stdout) = doc(&root)?;

    assert_eq!(code, EX_OK as i32);
    assert_eq!(stdout, "# Demo\n\nHow to use it.\n");

    Ok(())
}

#[test]
fn doc_fails_when_the_module_has_no_readme() -> Result {
    let root = TempDir::new()?;
    std::fs::create_dir_all(root.child("modules/installed/demo"))?;

    let (code, stdout) = doc(&root)?;

    assert_eq!(code, EX_UNAVAILABLE as i32);
    assert!(stdout.is_empty(), "should not print anything: {stdout}");

    Ok(())
}
