// This is free and unencumbered software released into the public domain.

use clientele::SysexitsError::*;
use std::process::{ExitStatus, Stdio};

use crate::shared::locate_subcommand;
use crate::Result;

pub struct ExternalResult {
    pub code: i32,
    pub stdout: Option<Vec<u8>>,
    pub stderr: Option<Vec<u8>>,
}

/// Executes the given subcommand.
pub struct External {
    pub is_debug: bool,
    pub pipe_output: bool,
}

impl External {
    pub fn execute(&self, cmd: &str, args: &[String]) -> Result<ExternalResult> {
        // Locate the given subcommand:
        let cmd = locate_subcommand(cmd)?;

        // Prepare the process:
        let result: std::io::Result<(ExitStatus, Option<Vec<u8>>, Option<Vec<u8>>)> =
            if self.pipe_output {
                std::process::Command::new(&cmd.path)
                    .args(args)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .map(|x| (x.status, Some(x.stdout), Some(x.stderr)))
            } else {
                std::process::Command::new(&cmd.path)
                    .args(args)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status()
                    .map(|x| (x, None, None))
            };

        match result {
            Err(error) => {
                if self.is_debug {
                    eprintln!("{}: {}", "asimov", error);
                }
                Err(EX_SOFTWARE)
            }
            Ok(result) => {
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;

                    if let Some(signal) = result.0.signal() {
                        if self.is_debug {
                            eprintln!("{}: terminated by signal {}", "asimov", signal);
                        }

                        return Ok(ExternalResult {
                            code: (signal | 0x80) & 0xff,
                            stdout: result.1,
                            stderr: result.2,
                        });
                    }
                }

                Ok(ExternalResult {
                    // unwrap_or should never happen because we are handling signal above.
                    code: result.0.code().unwrap_or(EX_SOFTWARE.as_i32()),
                    stdout: result.1,
                    stderr: result.2,
                })
            }
        }
    }
}
