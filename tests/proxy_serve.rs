// This is free and unencumbered software released into the public domain.

#![cfg(feature = "proxy")]

use clientele::SysexitsError::*;
use std::process::{Command, Stdio};

#[test]
fn occupied_proxy_ports_return_a_contextual_error() -> Result<(), Box<dyn core::error::Error>> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    let root = temp_dir::TempDir::new()?;
    let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
        .args([
            "proxy",
            "serve",
            "--bind",
            "127.0.0.1",
            "--port",
            &address.port().to_string(),
        ])
        .env("ASIMOV_ROOT", root.path())
        .current_dir(root.path())
        .env("OPENROUTER_API_KEY", "test-key")
        .env("no_proxy", "*")
        .env("NO_PROXY", "*")
        .env_remove("ASIMOV_PROXY_LOG_FILE")
        .stdin(Stdio::null())
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(EX_UNAVAILABLE as i32),
        "{stderr}"
    );
    assert!(stderr.contains("failed to bind proxy listener"), "{stderr}");
    assert!(stderr.contains(&address.to_string()));
    assert!(!stderr.contains("panicked"));
    Ok(())
}

#[test]
fn missing_and_invalid_credentials_fail_without_panics_or_disclosure()
-> Result<(), Box<dyn core::error::Error>> {
    let root = temp_dir::TempDir::new()?;
    for key in [None, Some(""), Some("private-secret\ninvalid")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_asimov"));
        command
            .args(["proxy", "serve"])
            .env("ASIMOV_ROOT", root.path())
            .current_dir(root.path())
            .env_remove("OPENROUTER_API_KEY")
            .stdin(Stdio::null());
        if let Some(key) = key {
            command.env("OPENROUTER_API_KEY", key);
        }
        let output = command.output()?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(EX_CONFIG as i32), "{stderr}");
        assert!(stderr.contains("OPENROUTER_API_KEY"));
        assert!(!stderr.contains("private-secret"));
        assert!(!stderr.contains("panicked"));
        assert!(output.stdout.is_empty());
    }
    Ok(())
}
