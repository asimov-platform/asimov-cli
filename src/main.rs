// This is free and unencumbered software released into the public domain.

#![deny(unsafe_code)]

mod feature;

use clientele::{
    crates::clap::{CommandFactory, Parser, Subcommand},
    exit, StandardOptions,
    SysexitsError::*,
};
use std::process::Stdio;

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
    #[clap(external_subcommand)]
    External(Vec<String>),
}

pub fn main() {
    // Load environment variables from `.env`:
    clientele::dotenv().ok();

    // Expand wildcards and @argfiles:
    let Ok(args) = clientele::args_os() else {
        exit(EX_USAGE);
    };

    // Parse command-line options:
    let options = Options::parse_from(&args);

    // Print the version, if requested:
    if options.flags.version {
        println!("ASIMOV {}", env!("CARGO_PKG_VERSION"));
        exit(EX_OK);
    }

    // Print the license, if requested:
    if options.flags.license {
        print!("{}", include_str!("../UNLICENSE"));
        exit(EX_OK);
    }

    // Configure debug output:
    if options.flags.debug {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    let commands = match asimov_cli::ExternalCommands::collect("asimov-", 1) {
        Ok(commands) => commands,
        Err(error) => {
            eprintln!(
                "{}: failed to collect external commands: {}",
                "asimov", error
            );
            return;
        }
    };

    // Print the help message, if requested:
    if options.help {
        let mut help = String::new();
        help.push_str(color_print::cstr!("<s><u>Commands:</u></s>\n"));
        for (i, cmd) in commands.iter().enumerate() {
            if i > 0 {
                help.push('\n');
            }
            help.push_str(&color_print::cformat!(
                "\t<dim>$</dim> <s>asimov {}</s> [OPTIONS] [COMMAND]",
                cmd.name,
            ));
        }
        Options::command()
            .after_long_help(help)
            .print_long_help()
            .unwrap();
        exit(EX_OK);
    }

    // Locate the given subcommand:
    let Command::External(command) = &options.command.unwrap();
    let Some(cmd_to_execute) = commands.find(&command[0]) else {
        eprintln!(
            "{}: command not found: {}{}",
            "asimov", "asimov-", command[0]
        );
        exit(EX_UNAVAILABLE);
    };

    // Execute the given subcommand:
    let status = std::process::Command::new(&cmd_to_execute.path)
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
            exit(EX_SOFTWARE);
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
                    exit((signal | 0x80) & 0xff);
                }
            }

            exit(status.code().unwrap_or(EX_SOFTWARE.as_i32()))
        }
    }
}
