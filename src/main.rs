// This is free and unencumbered software released into the public domain.

#![deny(unsafe_code)]
#![allow(unused)]

use asimov_cli::commands::{self, External, Help, HelpCmd};
use clientele::{
    crates::clap::{CommandFactory, Parser, Subcommand as ClapSubcommand},
    StandardOptions, SubcommandsProvider,
    SysexitsError::{self, *},
};

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
    /// Print help for a subcommand
    Help {
        #[clap(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Fetch raw data from a URL, utilizing enabled modules
    #[cfg(feature = "fetch")]
    Fetch { urls: Vec<String> },

    /// Import knowledge from a URL, utilizing enabled modules
    #[cfg(feature = "import")]
    Import { urls: Vec<String> },

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
    let Ok(options) = Options::try_parse_from(&args) else {
        print_help();
        return EX_OK;
    };

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
            if let Some(cmd_name) = args.first() {
                let cmd = HelpCmd {
                    is_debug: options.flags.debug,
                };

                let result = cmd.execute(cmd_name, &args[1..]);
                if let Ok(result) = &result {
                    if result.success {
                        let stdout = std::io::stdout();
                        let mut stdout = stdout.lock();
                        std::io::copy(&mut result.output.as_slice(), &mut stdout).unwrap();
                    } else {
                        eprintln!("{}: {} doesn't provide help", "asimov", cmd_name);

                        if options.flags.debug {
                            eprintln!("{}: status code - {}", "asimov", result.code);

                            let stdout = std::io::stdout();
                            let mut stdout = stdout.lock();
                            std::io::copy(&mut result.output.as_slice(), &mut stdout).unwrap();
                        }
                    }
                }

                result.map(|result| result.code)
            } else {
                print_full_help();
                Ok(EX_OK)
            }
        }
        #[cfg(feature = "fetch")]
        Command::Fetch { urls } => commands::fetch(urls, &options.flags).map(|_| EX_OK),
        #[cfg(feature = "import")]
        Command::Import { urls } => commands::import(urls, &options.flags).map(|_| EX_OK),
        Command::External(args) => {
            let cmd = External {
                is_debug: options.flags.debug,
                pipe_output: false,
            };

            cmd.execute(&args[0], &args[1..]).map(|result| result.code)
        }
    };

    // Return whatever status code we got.
    // NOTE: We could return Result<...> here, however
    // in that case we would get an annoying `Error: ...` message,
    // which is not what we want. So we just return an error like this.
    result.unwrap_or_else(|e| e)
}

/// Prints full help message.
fn print_full_help() {
    let mut help = String::new();
    help.push_str(color_print::cstr!("<s><u>Commands:</u></s>\n"));

    let cmds = Help.execute();
    for (i, cmd) in cmds.iter().enumerate() {
        if i > 0 {
            help.push_str("\n\n")
        }

        let predicted_usage = format!("Usage: asimov-{} ", cmd.name);

        let description = cmd.description.replace('\n', "\n\t");

        if let Some(usage) = cmd
            .usage
            .as_ref()
            .and_then(|usage| usage.strip_prefix(&predicted_usage))
        {
            // Usage string starts just as we expected. Skip it and print the arguments only.

            help.push_str(&color_print::cformat!(
                "\t<dim>$</dim> <s>asimov {}</s> {}\n\t{}",
                cmd.name,
                usage,
                description,
            ));
        } else {
            // Either usage unavailable or it doesn't start with the expected string,
            // fallback to the default message.

            help.push_str(&color_print::cformat!(
                "\t<dim>$</dim> <s>asimov {}</s> [OPTIONS] [COMMAND]\n\t{}",
                cmd.name,
                description
            ));
        }
    }

    Options::command()
        .after_long_help(help)
        .print_long_help()
        .unwrap();
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
