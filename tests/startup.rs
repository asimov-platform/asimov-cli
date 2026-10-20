// This is free and unencumbered software released into the public domain.

#![cfg(any(
    all(unix, feature = "module"),
    feature = "proxy",
    feature = "resource",
    feature = "telemetry"
))]

#[cfg(any(feature = "proxy", feature = "resource", feature = "telemetry"))]
use clientele::SysexitsError::*;
use std::process::{Command, Stdio};

#[test]
#[cfg(any(feature = "proxy", feature = "resource", feature = "telemetry"))]
fn independent_commands_do_not_initialize_module_or_snapshot_storage()
-> Result<(), Box<dyn core::error::Error>> {
    for blocked in [false, true] {
        for (args, expected) in [
            #[cfg(feature = "proxy")]
            (vec!["proxy", "url"], EX_OK),
            #[cfg(feature = "resource")]
            (vec!["resource", "list"], EX_OK),
            #[cfg(feature = "resource")]
            (vec!["list"], EX_OK),
            #[cfg(feature = "telemetry")]
            (vec!["configure", "telemetry", "disable"], EX_OK),
        ] {
            let root = temp_dir::TempDir::new()?;
            if blocked {
                for name in ["modules", "snapshots"] {
                    std::fs::write(root.child(name), b"keep")?;
                }
            }
            let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
                .args(&args)
                .env("ASIMOV_ROOT", root.path())
                .env("HOME", root.path())
                .env("USERPROFILE", root.path())
                .env("ASIMOV_KEYRING_BACKEND", "file")
                .env("DO_NOT_TRACK", "1")
                .current_dir(root.path())
                .stdin(Stdio::null())
                .output()?;
            assert_eq!(
                output.status.code(),
                Some(expected as i32),
                "{args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            if args.last() == Some(&"list") {
                assert!(output.stdout.is_empty());
                assert!(!root.child("resources").exists());
            }
            for name in ["modules", "snapshots"] {
                if blocked {
                    assert_eq!(std::fs::read(root.child(name))?, b"keep");
                } else {
                    assert!(!root.child(name).exists(), "{args:?} created {name}");
                }
            }
        }
    }
    Ok(())
}

#[cfg(all(unix, feature = "module"))]
#[test]
fn module_list_with_a_public_identity_does_not_access_the_secret_backend()
-> Result<(), Box<dyn core::error::Error>> {
    let home = temp_dir::TempDir::new()?;
    let root = home.child(".asimov");
    let directory = root.join("keyring");
    std::fs::create_dir_all(&directory)?;
    let user = whoami::username().unwrap_or_else(|_| "default".into());
    let public: asimov_id::PublicKey = iroh_base::SecretKey::from_bytes(&[42; 32]).public().into();
    let contents = format!("{public}\n");
    std::fs::write(directory.join(&user), &contents)?;

    for _ in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
            .args(["module", "list"])
            .env("HOME", home.path())
            .env("ASIMOV_ROOT", &root)
            // Any attempt to initialize a secret backend must fail this test.
            .env("ASIMOV_KEYRING_BACKEND", "invalid")
            .env("DO_NOT_TRACK", "1")
            .current_dir(home.path())
            .stdin(Stdio::null())
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(std::fs::read_to_string(directory.join(&user))?, contents);
        assert!(!root.join(".keyring").exists());
    }
    Ok(())
}
