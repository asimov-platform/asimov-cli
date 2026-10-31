// This is free and unencumbered software released into the public domain.

use clientele::SubcommandsProvider;
use rayon::prelude::*;
use std::{
    io::Read,
    process::{Child, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

use crate::Result;

pub struct CommandDescription {
    pub name: String,
    pub description: String,
    pub usage: Option<String>,
}

/// Prints extensive help message, executing `help` command for each subcommand.
pub struct Help;

impl Help {
    pub fn execute(&self) -> Vec<CommandDescription> {
        let output = self.collect_output();

        let mut result = vec![];
        for (name, description) in output {
            let lines = description.lines().collect::<Vec<_>>();
            let usage = lines.iter().find(|line| line.starts_with("Usage:"));

            // Parse description until the end or an empty line.
            let description = lines
                .iter()
                .map_while(|line| {
                    if !line.trim().is_empty() {
                        Some(line.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            result.push(CommandDescription {
                name,
                description,
                usage: usage.map(|usage| usage.to_string()),
            });
        }

        result
    }

    fn collect_output(&self) -> Vec<(String, String)> {
        let provider = SubcommandsProvider::collect("asimov-", 1);
        provider
            .get_commands()
            .par_iter()
            .filter_map(|cmd| Some((cmd.name.clone(), run_help(&cmd.path)?)))
            .collect()
    }
}

/// How long each command may take to print its help and exit.
const HELP_DEADLINE: Duration = Duration::from_secs(1);

/// How much of each command's help output is kept; the rest is discarded.
const HELP_OUTPUT_LIMIT: u64 = 256 * 1024;

/// Runs `<path> --help` and returns its stdout, or `None` if the command
/// can't be spawned, fails, or doesn't finish within [`HELP_DEADLINE`].
///
/// Stdout is drained while the child runs, so that verbose help can't block
/// on a full pipe; a child that times out is killed and reaped.
fn run_help(path: &std::path::Path) -> Option<String> {
    let deadline = Instant::now() + HELP_DEADLINE;
    let mut child = std::process::Command::new(path)
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    // The reader is detached rather than joined: if a grandchild inherits the
    // pipe and keeps it open, the reader outlives us instead of blocking us.
    let mut stdout = child.stdout.take()?;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut output = Vec::new();
        let result = (&mut stdout)
            .take(HELP_OUTPUT_LIMIT)
            .read_to_end(&mut output)
            .and_then(|_| std::io::copy(&mut stdout, &mut std::io::sink()));
        sender.send(result.map(|_| output)).ok();
    });

    let output = receiver
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .ok()
        .and_then(Result::ok);
    let success = output.is_some() && wait_until(&mut child, deadline);
    if !success {
        child.kill().ok();
        child.wait().ok();
        return None;
    }
    Some(String::from_utf8_lossy(&output?).into_owned())
}

/// Waits for the child to exit by the deadline, returning whether it exited
/// successfully. Stdout has already closed, so this is normally immediate.
fn wait_until(child: &mut Child, deadline: Instant) -> bool {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            },
            _ => return false,
        }
    }
}
