// This is free and unencumbered software released into the public domain.

#![cfg(feature = "proxy")]

use clientele::SysexitsError::*;
use std::process::{Command, Stdio};

#[test]
fn proxy_reporting_rejects_invalid_endpoint_environment() -> Result<(), Box<dyn core::error::Error>>
{
    let root = temp_dir::TempDir::new()?;
    for (name, value) in [
        ("ASIMOV_PROXY_BIND", "bad-address"),
        ("ASIMOV_PROXY_PORT", "65536"),
        ("ASIMOV_PROXY_PORT", ""),
    ] {
        for command in ["host", "port", "url"] {
            let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
                .args(["proxy", command])
                .env("ASIMOV_ROOT", root.path())
                .env_remove("ASIMOV_PROXY_BIND")
                .env_remove("ASIMOV_PROXY_PORT")
                .env(name, value)
                .current_dir(root.path())
                .stdin(Stdio::null())
                .output()?;
            assert_eq!(output.status.code(), Some(EX_CONFIG as i32));
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains(name));
        }
    }
    Ok(())
}

#[test]
fn proxy_urls_bracket_ipv6_addresses() -> Result<(), Box<dyn core::error::Error>> {
    let root = temp_dir::TempDir::new()?;
    for (host, port, expected) in [
        ("127.0.0.1", "1920", "http://127.0.0.1:1920/v1\n"),
        ("::1", "2020", "http://[::1]:2020/v1\n"),
        ("2001:db8::1", "8080", "http://[2001:db8::1]:8080/v1\n"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
            .args(["proxy", "url"])
            .env("ASIMOV_ROOT", root.path())
            .env("ASIMOV_PROXY_BIND", host)
            .env("ASIMOV_PROXY_PORT", port)
            .current_dir(root.path())
            .stdin(Stdio::null())
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let rendered = String::from_utf8(output.stdout)?;
        assert_eq!(rendered, expected);
        let parsed = url::Url::parse(rendered.trim())?;
        assert_eq!(parsed.port(), Some(port.parse()?));
    }
    Ok(())
}

#[test]
fn windows_assignment_formats_use_distinct_syntax() -> Result<(), Box<dyn core::error::Error>> {
    let root = temp_dir::TempDir::new()?;
    for (format, expected) in [
        (
            "set",
            "set \"OPENAI_API_BASE=http://127.0.0.1:1920/v1\"\nset \"OPENAI_API_KEY=powershell\"\n",
        ),
        (
            "setx",
            "setx OPENAI_API_BASE \"http://127.0.0.1:1920/v1\"\nsetx OPENAI_API_KEY \"powershell\"\n",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
            .args(["proxy", "config", "powershell", "--format", format])
            .env("ASIMOV_ROOT", root.path())
            .current_dir(root.path())
            .stdin(Stdio::null())
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8(output.stdout)?, expected);
    }
    Ok(())
}

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
