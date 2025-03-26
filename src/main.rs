// This is free and unencumbered software released into the public domain.

#![deny(unsafe_code)]

mod feature;

use clientele::{
    crates::clap::{CommandFactory, Parser, Subcommand},
    StandardOptions,
    SysexitsError::{self, *},
};
use std::process::Stdio;

use asimov_cli::{ExternalCommand, ExternalCommands};

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

#[derive(Debug, Subcommand)]
enum Command {
    // FIXME: `help` command is not listed in the help message.
    Help {
        #[clap(trailing_var_arg = true)]
        args: Vec<String>,
    },
    #[clap(external_subcommand)]
    External(Vec<String>),
}

type Result<T = SysexitsError, E = SysexitsError> = std::result::Result<T, E>;

pub fn main() -> Result {
    // Load environment variables from `.env`:
    clientele::dotenv().ok();

    // Expand wildcards and @argfiles:
    let Ok(args) = clientele::args_os() else {
        return Err(EX_USAGE);
    };

    // Parse command-line options:
    let options = Options::parse_from(&args);

    // Print the version, if requested:
    if options.flags.version {
        println!("ASIMOV {}", env!("CARGO_PKG_VERSION"));
        return Ok(EX_OK);
    }

    // Print the license, if requested:
    if options.flags.license {
        print!("{}", include_str!("../UNLICENSE"));
        return Ok(EX_OK);
    }

    // Configure debug output:
    if options.flags.debug {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    // Print the help message, if requested:
    if options.help {
        return execute_help();
    }

    match options.command.as_ref().unwrap() {
        Command::Help { args } => {
            if args.is_empty() {
                execute_extensive_help()
            } else {
                execute_help_command(&options, args)
            }
        }
        Command::External(command) => execute_external_command(&options, command),
    }
}

/// Prints basic help message.
fn execute_help() -> Result {
    let mut help = String::new();
    help.push_str(color_print::cstr!("<s><u>Commands:</u></s>\n"));

    let commands = ExternalCommands::collect("asimov-", 1);
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

    Ok(EX_OK)
}

/// Prints extensive help message, executing `help` command for each subcommand.
fn execute_extensive_help() -> Result {
    unimplemented!()
}

/// Locates the given subcommand or prints an error.
fn locate_subcommand(name: &str) -> Result<ExternalCommand> {
    match ExternalCommands::find("asimov-", name) {
        Some(cmd) => Ok(cmd),
        None => {
            eprintln!("{}: command not found: {}{}", "asimov", "asimov-", name);
            Err(EX_UNAVAILABLE)
        }
    }
}

fn execute_subcommand(options: &Options, cmd: &ExternalCommand, args: &[String]) -> Result {
    let status = std::process::Command::new(&cmd.path)
        .args(args)
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

/// Executes `help` command for the given subcommand.
fn execute_help_command(options: &Options, command: &[String]) -> Result {
    assert!(!command.is_empty());

    // Locate the given subcommand:
    let cmd = locate_subcommand(&command[0])?;

    // Execute the `help` command:
    execute_subcommand(
        options,
        &cmd,
        &[&[String::from("help")], &command[1..]].concat(),
    )
}

/// Executes the given subcommand.
fn execute_external_command(options: &Options, command: &[String]) -> Result {
    assert!(!command.is_empty());

    // Locate the given subcommand:
    let cmd = locate_subcommand(&command[0])?;

    // Execute the given subcommand:
    execute_subcommand(options, &cmd, &command[1..])
}
