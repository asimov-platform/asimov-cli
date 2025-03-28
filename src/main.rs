// This is free and unencumbered software released into the public domain.

#![deny(unsafe_code)]

mod feature;

use clientele::{
    crates::clap::{CommandFactory, Parser, Subcommand as ClapSubcommand},
    StandardOptions,
    SysexitsError::{self, *},
};
use std::process::Stdio;

use asimov_cli::{Result, Subcommand, SubcommandsProvider};

/// ASIMOV Command-Line Interface (CLI)
#[derive(Debug, Parser)]
#[command(name = "asimov", long_about)]
#[command(allow_external_subcommands = true)]
#[command(arg_required_else_help = true)]
#[command(disable_help_flag = true)]
#[command(disable_help_subcommand = true)]
struct Options {
    #[clap(flatten)]
    flags: StandardOptions,

    #[clap(short = 'h', long, help = "Print help (see more with '--help')")]
    help: bool,

    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, ClapSubcommand)]
enum Command {
    // FIXME: `help` command is not listed in the help message.
    Help {
        #[clap(trailing_var_arg = true)]
        args: Vec<String>,
    },
    #[clap(external_subcommand)]
    External(Vec<String>),
}

pub fn main() -> SysexitsError {
    // Load environment variables from `.env`:
    clientele::dotenv().ok();

    // Expand wildcards and @argfiles:
    let Ok(args) = clientele::args_os() else {
        return EX_USAGE;
    };

    // Parse command-line options:
    let options = Options::parse_from(&args);

    // Print the version, if requested:
    if options.flags.version {
        println!("ASIMOV {}", env!("CARGO_PKG_VERSION"));
        return EX_OK;
    }

    // Print the license, if requested:
    if options.flags.license {
        print!("{}", include_str!("../UNLICENSE"));
        return EX_OK;
    }

    // Configure debug output:
    if options.flags.debug {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    // Print the help message, if requested:
    if options.help {
        print_help();
        return EX_OK;
    }

    let result = match options.command.as_ref().unwrap() {
        Command::Help { args } => {
            if args.is_empty() {
                execute_extensive_help()
            } else {
                execute_help_command(&options, args)
            }
        }
        Command::External(command) => execute_external_command(&options, command),
    };

    // We can handle result here if we want.
    match result {
        Ok(code) => code,
        Err(code) => code,
    }
}

/// Prints basic help message.
fn print_help() {
    let mut help = String::new();
    help.push_str(color_print::cstr!("<s><u>Commands:</u></s>\n"));

    let commands = SubcommandsProvider::collect("asimov-", 1);
    for (i, cmd) in commands.iter().enumerate() {
        if i > 0 {
            help.push('\n');
        }
        println!("{}", cmd.path.display());
        help.push_str(&color_print::cformat!(
            "\t<dim>$</dim> <s>asimov {}</s> [OPTIONS] [COMMAND]",
            cmd.name,
        ));
    }

    Options::command()
        .after_long_help(help)
        .print_long_help()
        .unwrap();
}

/// Prints extensive help message, executing `help` command for each subcommand.
fn execute_extensive_help() -> Result {
    unimplemented!()
}

/// Locates the given subcommand or prints an error.
fn locate_subcommand(name: &str) -> Result<Subcommand> {
    match SubcommandsProvider::find("asimov-", name) {
        Some(cmd) => Ok(cmd),
        None => {
            eprintln!("{}: command not found: {}{}", "asimov", "asimov-", name);
            Err(EX_UNAVAILABLE)
        }
    }
}

/// Executes `help` command for the given subcommand.
fn execute_help_command(options: &Options, command: &[String]) -> Result {
    assert!(!command.is_empty());

    // Locate the given subcommand:
    let cmd = locate_subcommand(&command[0])?;

    // Execute the `--help` command:
    let output = std::process::Command::new(&cmd.path)
        .args([&[String::from("--help")], &command[1..]].concat())
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Err(error) => {
            if options.flags.debug {
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

                if options.flags.debug {
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

/// Executes the given subcommand.
fn execute_external_command(options: &Options, command: &[String]) -> Result {
    assert!(!command.is_empty());

    // Locate the given subcommand:
    let cmd = locate_subcommand(&command[0])?;

    // Execute the given subcommand:
    let status = std::process::Command::new(&cmd.path)
        .args(&command[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    match status {
        Err(error) => {
            if options.flags.debug {
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
                    if options.flags.debug {
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
