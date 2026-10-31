// This is free and unencumbered software released into the public domain.

#![cfg(all(unix, feature = "resource"))]

use std::{
    os::unix::fs::{PermissionsExt, symlink},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use temp_dir::TempDir;

type Result<T = ()> = core::result::Result<T, Box<dyn core::error::Error>>;

/// Records each fetcher's PID and how many fetchers were running when it
/// started. The URL's last segment sets the delay: later digits finish
/// sooner, so output order can't follow completion order; `hang` blocks.
const FETCHER: &str = r#"#!/bin/sh
for url; do :; done
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
                "provides": {"programs": ["asimov-slow-fetcher"]},
                "handles": {"url_prefixes": ["https://slow.example/"]}
            }"#,
        )?;
        std::fs::create_dir_all(root.child("modules/enabled"))?;
        symlink(&installed, root.child("modules/enabled/slow"))?;
        std::fs::create_dir_all(root.child("bin"))?;
        std::fs::create_dir_all(root.child("log"))?;
        let path = root.child("bin/asimov-slow-fetcher");
        std::fs::write(&path, FETCHER)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        Ok(Self(root))
    }

    fn fetch(&self, segments: &[String]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_asimov"));
        command
            .args(["resource", "fetch"])
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
}

fn jobs() -> usize {
    std::thread::available_parallelism().map_or(1, core::num::NonZero::get)
}

#[test]
fn fetch_runs_at_most_one_fetcher_per_cpu_in_input_order() -> Result {
    let sandbox = Sandbox::new()?;
    let segments: Vec<String> = (0..2 * jobs() + 1).map(|n| n.to_string()).collect();
    let output = sandbox.fetch(&segments).output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let fetched: Vec<String> = String::from_utf8(output.stdout)?
        .lines()
        .map(|line| Ok(serde_json::from_str::<serde_json::Value>(line)?["n"].to_string()))
        .collect::<Result<_>>()?;
    assert_eq!(fetched, segments);

    let peaks = sandbox.log("peaks")?;
    assert_eq!(peaks.len(), segments.len());
    let peak = peaks
        .iter()
        .map(|n| n.parse::<usize>())
        .collect::<core::result::Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or_default();
    assert!(
        peak <= jobs(),
        "{peak} fetchers ran at once, over {}",
        jobs()
    );
    Ok(())
}

#[test]
fn fetch_stops_running_fetchers_when_stdout_closes() -> Result {
    let sandbox = Sandbox::new()?;
    let segments = ["0".into(), "hang".into()];
    let (reader, writer) = std::io::pipe()?;
    drop(reader);
    let started = Instant::now();
    let status = sandbox.fetch(&segments).stdout(writer).status()?;
    assert!(status.success(), "{status}");
    assert!(started.elapsed() < Duration::from_secs(20));

    for pid in sandbox.log("pids")? {
        let alive = Command::new("/bin/sh")
            .args(["-c", &format!("kill -0 {pid} 2>/dev/null")])
            .status()?
            .success();
        assert!(!alive, "fetcher {pid} outlived `asimov resource fetch`");
    }
    Ok(())
}
