// This is free and unencumbered software released into the public domain.

use clientele::SysexitsError::*;
use std::process::Stdio;

use crate::shared::locate_subcommand;
use crate::Result;

/// Executes `help` command for the given subcommand.
pub struct HelpCmdCommand {
    pub is_debug: bool,
}

impl HelpCmdCommand {
    pub fn execute(&self, cmd: &str, args: &[String]) -> Result {
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
                Some(code) if code == EX_OK.as_i32() => {
                    use std::process::exit;

                    let stdout = std::io::stdout();
                    let mut stdout = stdout.lock();
                    std::io::copy(&mut output.stdout.as_slice(), &mut stdout).unwrap();

                    exit(output.status.code().unwrap_or(EX_SOFTWARE.as_i32()))
                }
                _ => {
                    eprintln!("{}: {} doesn't provide help", "asimov", cmd.name);

                    if self.is_debug {
                        eprintln!("{}: status code - {}", "asimov", output.status);

                        let stdout = std::io::stdout();
                        let mut stdout = stdout.lock();
                        std::io::copy(&mut output.stderr.as_slice(), &mut stdout).unwrap();
                    }

                    Err(EX_SOFTWARE)
                }
            },
        }
    }
}
