// This is free and unencumbered software released into the public domain.

#![cfg(feature = "proxy")]

use clientele::SysexitsError::*;
use std::process::{Command, Stdio};

#[test]
fn config_formats_are_validated_for_each_target() -> Result<(), Box<dyn core::error::Error>> {
    let root = temp_dir::TempDir::new()?;
    for (target, format, success) in [
        ("bash", "export", true),
        ("zed", "json", true),
        ("langchain", "py", true),
        ("bash", "json", false),
        ("zed", "export", false),
        ("aider", "json", false),
        ("dotenv", "json", false),
        ("goose", "env", false),
        ("langchain", "env", false),
        ("litellm", "env", false),
        ("llamaindex", "json", false),
        ("opencode", "env", false),
        ("openhands", "json", false),
        ("powershell", "json", false),
        ("pi", "env", false),
        ("zsh", "json", false),
        ("bash", "typo", false),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
            .args(["proxy", "config", target, "--format", format])
            .env("ASIMOV_ROOT", root.path())
            .current_dir(root.path())
            .stdin(Stdio::null())
            .output()?;
        // Clap rejects unknown formats with 2; target mismatches use EX_USAGE.
        let expected = if success {
            EX_OK as i32
        } else if format == "typo" {
            2
        } else {
            EX_USAGE as i32
        };
        assert_eq!(
            output.status.code(),
            Some(expected),
            "{target}/{format}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(!output.stdout.is_empty(), success, "{target}/{format}");
        if !success {
            assert!(!output.stderr.is_empty(), "{target}/{format}");
        }
    }
    Ok(())
}
