// This is free and unencumbered software released into the public domain.

use clientele::SysexitsError::{self, *};
use std::process::Stdio;

use crate::{Result, shared::locate_subcommand};

pub struct HelpCmdResult {
    /// Whether the command was successful or not.
    pub success: bool,

    /// Return code of the executed command.
    pub code: SysexitsError,

    /// If `success` is `true`, this field contains stdout,
    /// otherwise it contains stderr.
    pub output: Vec<u8>,
}

/// Executes `help` command for the given subcommand.
pub struct HelpCmd {
    pub is_debug: bool,
}

impl HelpCmd {
    pub fn execute(&self, cmd: &str, args: &[String]) -> Result<HelpCmdResult> {
        // Locate the given subcommand:
        let cmd = locate_subcommand(cmd)?;

        // Execute the `--help` command:
        let output = std::process::Command::new(&cmd.path)
            .args([&[String::from("--help")], args].concat())
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match output {
            Err(error) => {
                if self.is_debug {
                    eprintln!("{}: {}", "asimov", error);
                }
                Err(EX_SOFTWARE)
            }
            Ok(output) => match output.status.code() {
                Some(code) if code == EX_OK.as_i32() => Ok(HelpCmdResult {
                    success: true,
                    code: EX_OK,
                    output: output.stdout,
                }),
                _ => Ok(HelpCmdResult {
                    success: false,
                    code: output
                        .status
                        .code()
                        .and_then(|code| SysexitsError::try_from(code).ok())
                        .unwrap_or(EX_SOFTWARE),
                    output: output.stderr,
                }),
            },
        }
    }
}
