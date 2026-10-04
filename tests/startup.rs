// This is free and unencumbered software released into the public domain.

#![cfg(any(feature = "proxy", feature = "telemetry"))]

use std::process::{Command, Stdio};

#[test]
fn independent_commands_do_not_initialize_module_or_snapshot_storage()
-> Result<(), Box<dyn core::error::Error>> {
    for blocked in [false, true] {
        for args in [
            #[cfg(feature = "proxy")]
            vec!["proxy", "url"],
            #[cfg(feature = "telemetry")]
            vec!["configure", "telemetry", "disable"],
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
                .current_dir(root.path())
                .stdin(Stdio::null())
                .output()?;
            assert!(
                output.status.success(),
                "{args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
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
