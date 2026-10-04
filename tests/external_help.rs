// This is free and unencumbered software released into the public domain.

#![cfg(unix)]

use std::{
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};

#[test]
fn external_summaries_are_collected_only_for_root_long_help()
-> Result<(), Box<dyn core::error::Error>> {
    let root = temp_dir::TempDir::new()?;
    let program = root.child("asimov-probe");
    let marker = root.child("called");
    std::fs::write(
        &program,
        "#!/bin/sh\nif [ \"$1\" = --help ]; then\nprintf x >> \"$HELP_MARKER\"\nprintf 'Probe description\\n\\nUsage: asimov-probe [OPTIONS]\\n'\nfi\n",
    )?;
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))?;
    for (args, collect) in [
        (vec!["--version"], false),
        (vec!["--license"], false),
        (vec!["-h"], false),
        (vec!["probe", "argument"], false),
        #[cfg(feature = "module")]
        (vec!["module", "--help"], false),
        (vec!["--help"], true),
        (vec!["--color", "never", "--help"], true),
        #[cfg(any(
            feature = "module",
            feature = "proxy",
            feature = "source",
            feature = "telemetry"
        ))]
        (vec!["help"], true),
    ] {
        if marker.exists() {
            std::fs::remove_file(&marker)?;
        }
        let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
            .args(&args)
            .env("PATH", root.path())
            .env("ASIMOV_ROOT", root.path())
            .env("HELP_MARKER", &marker)
            .current_dir(root.path())
            .stdin(Stdio::null())
            .output()?;
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(marker.exists(), collect, "{args:?}");
        if collect {
            assert_eq!(std::fs::read(&marker)?, b"x");
            assert!(String::from_utf8_lossy(&output.stdout).contains("Probe description"));
        }
    }
    Ok(())
}

#[test]
#[cfg(any(
    feature = "module",
    feature = "proxy",
    feature = "source",
    feature = "telemetry"
))]
fn nested_external_help_places_the_help_flag_after_subcommands()
-> Result<(), Box<dyn core::error::Error>> {
    let root = temp_dir::TempDir::new()?;
    let program = root.child("asimov-probe");
    std::fs::write(&program, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n")?;
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))?;
    for (args, expected) in [
        (vec!["help", "probe"], "--help\n"),
        (vec!["--color", "never", "help", "probe"], "--help\n"),
        (
            vec!["-v", "--color=never", "help", "probe", "nested"],
            "nested\n--help\n",
        ),
        (
            vec!["help", "probe", "nested", "child"],
            "nested\nchild\n--help\n",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_asimov"))
            .args(args)
            .env("PATH", root.path())
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
