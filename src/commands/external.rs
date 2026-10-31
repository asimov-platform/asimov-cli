// This is free and unencumbered software released into the public domain.

use clientele::SysexitsError::*;
use std::ffi::OsString;
use std::process::Stdio;

use crate::{Result, shared::locate_subcommand};

pub struct ExternalResult {
    /// Exit code of the executed command, or 128 plus the signal number if
    /// it was terminated by a signal. Codes that don't fit in a byte (e.g.,
    /// Windows NTSTATUS values) are reported as `EX_SOFTWARE`.
    pub code: u8,

    /// If `pipe_output` is `true`, this field contains stdout, otherwise its None.
    pub stdout: Option<Vec<u8>>,

    /// If `pipe_output` is `true`, this field contains stderr, otherwise its None.
    pub stderr: Option<Vec<u8>>,
}

/// Executes the given subcommand.
pub struct ExternalSubcommand {
    pub is_debug: bool,
    pub pipe_output: bool,
}

impl ExternalSubcommand {
    pub fn execute(&self, cmd: &str, args: impl AsRef<[String]>) -> Result<ExternalResult> {
        self.execute_os(
            cmd,
            args.as_ref().iter().map(OsString::from).collect::<Vec<_>>(),
        )
    }

    /// Executes an external command without requiring UTF-8 argument values.
    pub fn execute_os(&self, cmd: &str, args: impl AsRef<[OsString]>) -> Result<ExternalResult> {
        // Locate the given subcommand:
        let cmd = locate_subcommand(cmd)?;

        // Prepare the process:
        let result = if self.pipe_output {
            std::process::Command::new(&cmd.path)
                .args(args.as_ref())
                .stdin(Stdio::inherit())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .map(|x| (x.status, Some(x.stdout), Some(x.stderr)))
        } else {
            std::process::Command::new(&cmd.path)
                .args(args.as_ref())
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .map(|x| (x, None, None))
        };

        match result {
            Err(error) => {
                if self.is_debug {
                    eprintln!("asimov: {}", error);
                }
                Err(EX_SOFTWARE)
            },
            Ok(result) => {
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;

                    if let Some(signal) = result.0.signal() {
                        if self.is_debug {
                            eprintln!("asimov: terminated by signal {}", signal);
                        }

                        return Ok(ExternalResult {
                            code: u8::try_from(128 + signal).unwrap_or(EX_SOFTWARE.as_u8()),
                            stdout: result.1,
                            stderr: result.2,
                        });
                    }
                }

                Ok(ExternalResult {
                    // `code()` is only `None` for signals, handled above.
                    code: result
                        .0
                        .code()
                        .and_then(|code| u8::try_from(code).ok())
                        .unwrap_or(EX_SOFTWARE.as_u8()),
                    stdout: result.1,
                    stderr: result.2,
                })
            },
        }
    }
}
