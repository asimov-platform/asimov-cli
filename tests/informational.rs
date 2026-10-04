// This is free and unencumbered software released into the public domain.

use std::process::{Command, Stdio};

#[test]
fn informational_commands_do_not_require_identity_storage()
-> Result<(), Box<dyn core::error::Error>> {
    let root = temp_dir::TempDir::new()?;
    let blocked = root.child("not-a-directory");
    std::fs::write(&blocked, b"unchanged")?;
    for args in [
        vec!["--help"],
        vec!["--version"],
        vec!["--license"],
        #[cfg(feature = "module")]
        vec!["module", "--help"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
            .args(&args)
            .env("ASIMOV_ROOT", &blocked)
            .env("PATH", root.path())
            .current_dir(root.path())
            .stdin(Stdio::null())
            .output()?;
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.stdout.is_empty(), "{args:?}");
        assert_eq!(std::fs::read(&blocked)?, b"unchanged");
    }
    Ok(())
}
