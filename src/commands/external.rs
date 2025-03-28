// This is free and unencumbered software released into the public domain.

use clientele::SysexitsError::*;
use std::process::Stdio;

use crate::shared::locate_subcommand;
use crate::Result;

/// Executes the given subcommand.
pub struct ExternalCmd {
    pub is_debug: bool,
}

impl ExternalCmd {
    pub fn execute(&self, cmd: &str, args: &[String]) -> Result<i32> {
        // Locate the given subcommand:
        let cmd = locate_subcommand(cmd)?;

        // Execute the given subcommand:
        let status = std::process::Command::new(&cmd.path)
            .args(args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();

        match status {
            Err(error) => {
                if self.is_debug {
                    eprintln!("{}: {}", "asimov", error);
                }
                Err(EX_SOFTWARE)
            }
            Ok(status) => {
                use std::process::exit;

                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;

                    if let Some(signal) = status.signal() {
                        if self.is_debug {
                            eprintln!("{}: terminated by signal {}", "asimov", signal);
                        }
                        exit((signal | 0x80) & 0xff)
                    }
                }

                // unwrap_or should never happen because we are handling signal above.
                exit(status.code().unwrap_or(EX_SOFTWARE.as_i32()))
            }
        }
    }
}
