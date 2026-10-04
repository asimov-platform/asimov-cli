// This is free and unencumbered software released into the public domain.

#![cfg(feature = "proxy")]

use clientele::SysexitsError::*;
use std::process::{Command, Stdio};

#[cfg(unix)]
#[tokio::test]
async fn termination_signals_stop_the_proxy_cleanly() -> Result<(), Box<dyn core::error::Error>> {
    use std::time::Duration;
    use tokio::io::AsyncBufReadExt;
    for signal in ["-INT", "-TERM"] {
        let root = temp_dir::TempDir::new()?;
        let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_asimov"))
            .args(["-v", "proxy", "serve", "--bind", "127.0.0.1", "--port", "0"])
            .env("ASIMOV_ROOT", root.path())
            .current_dir(root.path())
            .env("OPENROUTER_API_KEY", "test-key")
            .env("ASIMOV_TELEMETRY", "0")
            .env("no_proxy", "*")
            .env("NO_PROXY", "*")
            .env_remove("ASIMOV_PROXY_LOG_FILE")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let mut lines = tokio::io::BufReader::new(child.stderr.take().unwrap()).lines();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let line = lines
                    .next_line()
                    .await?
                    .ok_or("proxy exited before listening")?;
                if line.starts_with("Listening on ") {
                    break;
                }
            }
            Ok::<_, Box<dyn core::error::Error>>(())
        })
        .await??;
        let status = Command::new("/bin/kill")
            .args([signal, &child.id().unwrap().to_string()])
            .status()?;
        assert!(status.success());
        let status = tokio::time::timeout(Duration::from_secs(5), child.wait()).await??;
        assert!(status.success(), "{signal}: {status}");
    }
    Ok(())
}

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
        .env("ASIMOV_PROXY_BIND", "invalid-but-overridden")
        .env("ASIMOV_PROXY_PORT", "invalid-but-overridden")
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
