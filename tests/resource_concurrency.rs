// This is free and unencumbered software released into the public domain.

#![cfg(all(unix, feature = "resource"))]

use std::{
    os::unix::fs::{PermissionsExt, symlink},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use temp_dir::TempDir;

type Result<T = ()> = core::result::Result<T, Box<dyn core::error::Error>>;

/// Serves as both fetcher and lister. Records each process's PID and how
/// many were running when it started. The URL's last segment sets the delay:
/// later digits finish sooner, so output order can't follow completion
/// order; `hang` blocks.
const MODULE: &str = r#"#!/bin/sh
for arg; do case $arg in https://slow.example/*) url=$arg;; esac; done
n=${url##*/}
echo $$ >> "$FETCH_LOG/pids"
/bin/mkdir "$FETCH_LOG/running.$$"
set -- "$FETCH_LOG"/running.*
echo $# >> "$FETCH_LOG/peaks"
if [ "$n" = hang ]; then exec /bin/sleep 30; fi
/bin/sleep "0.$((9 - n % 10))"
/bin/rmdir "$FETCH_LOG/running.$$"
printf '{"n":%s}\n' "$n"
"#;

struct Sandbox(TempDir);

impl Sandbox {
    fn new() -> Result<Self> {
        let root = TempDir::new()?;
        let installed = root.child("modules/installed/slow");
        std::fs::create_dir_all(&installed)?;
        std::fs::write(
            installed.join("manifest.json"),
            r#"{
                "name": "slow",
                "provides": {"programs": ["asimov-slow-fetcher", "asimov-slow-lister"]},
                "handles": {"url_prefixes": ["https://slow.example/"]}
            }"#,
        )?;
        std::fs::create_dir_all(root.child("modules/enabled"))?;
        symlink(&installed, root.child("modules/enabled/slow"))?;
        std::fs::create_dir_all(root.child("bin"))?;
        std::fs::create_dir_all(root.child("log"))?;
        for program in ["fetcher", "lister"] {
            let path = root.child(format!("bin/asimov-slow-{program}"));
            std::fs::write(&path, MODULE)?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        }
        Ok(Self(root))
    }

    /// Runs `asimov resource <operation> <options>` on the URL segments.
    fn run(&self, operation: &str, options: &[&str], segments: &[String]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_asimov"));
        command
            .args(["resource", operation])
            .args(options)
            .args(segments.iter().map(|s| format!("https://slow.example/{s}")))
            .env("ASIMOV_ROOT", self.0.path())
            .env("HOME", self.0.path())
            .env("ASIMOV_KEYRING_BACKEND", "file")
            .env("DO_NOT_TRACK", "1")
            .env_remove("TYPESAFE_API_TOKEN")
            .env_remove("ASIMOV_RELAY_URL")
            .env("PATH", self.0.child("bin"))
            .env("FETCH_LOG", self.0.child("log"))
            .current_dir(self.0.path())
            .stdin(Stdio::null());
        command
    }

    fn log(&self, name: &str) -> Result<Vec<String>> {
        let log = std::fs::read_to_string(self.0.child("log").join(name))?;
        Ok(log.lines().map(String::from).collect())
    }

    /// The most modules seen running at once.
    fn peak(&self) -> Result<usize> {
        let mut peak = 0;
        for line in self.log("peaks")? {
            peak = peak.max(line.parse()?);
        }
        Ok(peak)
    }
}

fn jobs() -> usize {
    std::thread::available_parallelism().map_or(1, core::num::NonZero::get)
}

const OPERATIONS: &[&str] = &["fetch", "list"];

#[test]
fn urls_are_processed_at_most_one_per_cpu_in_input_order() -> Result {
    for operation in OPERATIONS {
        let sandbox = Sandbox::new()?;
        let segments: Vec<String> = (0..2 * jobs() + 1).map(|n| n.to_string()).collect();
        let output = sandbox.run(operation, &[], &segments).output()?;
        assert!(
            output.status.success(),
            "{operation}: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let emitted: Vec<String> = String::from_utf8(output.stdout)?
            .lines()
            .map(|line| Ok(serde_json::from_str::<serde_json::Value>(line)?["n"].to_string()))
            .collect::<Result<_>>()?;
        assert_eq!(emitted, segments, "{operation}");

        let peaks = sandbox.log("peaks")?;
        assert_eq!(peaks.len(), segments.len(), "{operation}");
        let peak = sandbox.peak()?;
        assert!(
            peak <= jobs(),
            "{operation}: {peak} URLs processed at once, over {}",
            jobs()
        );
    }
    Ok(())
}

#[test]
fn modules_stop_running_when_stdout_closes() -> Result {
    for operation in OPERATIONS {
        let sandbox = Sandbox::new()?;
        let segments = ["0".into(), "hang".into()];
        let (reader, writer) = std::io::pipe()?;
        drop(reader);
        let started = Instant::now();
        let status = sandbox
            .run(operation, &[], &segments)
            .stdout(writer)
            .status()?;
        assert!(status.success(), "{operation}: {status}");
        assert!(started.elapsed() < Duration::from_secs(20), "{operation}");

        for pid in sandbox.log("pids")? {
            let alive = Command::new("/bin/sh")
                .args(["-c", &format!("kill -0 {pid} 2>/dev/null")])
                .status()?
                .success();
            assert!(
                !alive,
                "module {pid} outlived `asimov resource {operation}`"
            );
        }
    }
    Ok(())
}

#[test]
fn jobs_option_overrides_the_per_cpu_default() -> Result {
    // Digits 5 to 9 keep delays short, as `-j 1` runs them one at a time.
    let segments: Vec<String> = (5..10).map(|n| n.to_string()).collect();
    for operation in OPERATIONS {
        for (options, jobs) in [(&["-j", "1"][..], 1), (&["--jobs=2"][..], 2)] {
            let sandbox = Sandbox::new()?;
            let output = sandbox.run(operation, options, &segments).output()?;
            assert!(
                output.status.success(),
                "{operation} {options:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let lines = String::from_utf8(output.stdout)?.lines().count();
            assert_eq!(lines, segments.len(), "{operation} {options:?}");
            let peak = sandbox.peak()?;
            assert!(
                peak <= jobs,
                "{operation} {options:?}: {peak} URLs processed at once"
            );
        }

        let sandbox = Sandbox::new()?;
        let output = sandbox.run(operation, &["-j", "0"], &segments).output()?;
        // Like other invalid option values, this is a Clap usage error.
        assert_eq!(output.status.code(), Some(2), "{operation}: -j 0");
        assert!(String::from_utf8(output.stderr)?.contains("--jobs"));
        assert!(
            sandbox.log("pids").is_err(),
            "{operation}: -j 0 ran modules"
        );
    }
    Ok(())
}
