// This is free and unencumbered software released into the public domain.

#![deny(unsafe_code)]
#![allow(unused)]

use asimov_cli::commands::{self, External, Help, HelpCmd};
use clientele::{
    StandardOptions, SubcommandsProvider,
    SysexitsError::{self, *},
    crates::clap::{CommandFactory, Parser, Subcommand as ClapSubcommand},
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

    /// Save a snapshot for a URL, utilizing enabled modules
    #[cfg(feature = "ask")]
    Ask {
        #[clap(long, short = 'm')]
        module: Option<String>,

        input: Option<String>,
    },

    /// TBD
    #[cfg(feature = "describe")]
    #[command(aliases = ["summarize", "tldr"])]
    Describe {
        #[clap(long, short = 'm')]
        module: Option<String>,

        /// The output format.
        #[arg(value_name = "FORMAT", short = 'o', long)]
        output: Option<String>,

        urls: Vec<String>,
    },

    /// Fetch knowledge from a URL, utilizing enabled modules
    #[cfg(feature = "fetch")]
    #[command(aliases = ["extract", "get", "import", "parse"])]
    Fetch {
        /// Optionally choose the module instead of using module resolution.
        /// The module's manifest must declare support for the URL for the
        /// module to be used.
        #[clap(long, short = 'm')]
        module: Option<String>,

        /// The output format.
        #[arg(value_name = "FORMAT", short = 'o', long)]
        output: Option<String>,

        urls: Vec<String>,
    },

    /// TBD
    #[cfg(feature = "index")]
    Index {
        #[clap(long, short = 'm')]
        module: Option<String>,

        urls: Vec<String>,
    },

    /// TBD
    #[cfg(feature = "list")]
    #[command(aliases = ["dir", "ls"])]
    List {
        #[clap(long, short = 'm')]
        module: Option<String>,

        /// The maximum number of resources to list.
        #[arg(value_name = "COUNT", short = 'n', long)]
        limit: Option<usize>,

        /// The output format.
        #[arg(value_name = "FORMAT", short = 'o', long)]
        output: Option<String>,

        urls: Vec<String>,
    },

    /// TBD
    #[cfg(feature = "read")]
    Read {
        #[clap(long, short = 'm')]
        module: Option<String>,

        urls: Vec<String>,
    },

    /// TBD
    #[cfg(feature = "search")]
    Search {
        #[clap(long, short = 'm')]
        module: Option<String>,

        prompt: String,
    },

    /// Save a snapshot for a URL, utilizing enabled modules
    #[cfg(feature = "snap")]
    Snap { urls: Vec<String> },

    #[clap(external_subcommand)]
    External(Vec<String>),
}

#[tokio::main]
pub async fn main() -> SysexitsError {
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

    asimov_module::init_tracing_subscriber(&options.flags).expect("failed to initialize logging");

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
        //std::env::set_var("RUST_BACKTRACE", "1");
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
                        eprintln!("asimov: {} doesn't provide help", cmd_name);

                        if options.flags.debug {
                            eprintln!("asimov: status code - {}", result.code);

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
        },

        #[cfg(feature = "ask")]
        Command::Ask { module, input } => {
            let input = if let Some(input) = input {
                input.clone()
            } else {
                use std::io::Read;
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf).unwrap();
                buf
            };
            commands::ask::ask(input, module.as_deref(), &options.flags)
                .await
                .map(|_| EX_OK)
        },

        #[cfg(feature = "describe")]
        Command::Describe {
            module,
            output,
            urls,
        } => {
            commands::describe::describe(urls, module.as_deref(), output.as_deref(), &options.flags)
                .await
                .map(|_| EX_OK)
        },

        #[cfg(feature = "fetch")]
        Command::Fetch {
            module,
            output,
            urls,
        } => commands::fetch::fetch(urls, module.as_deref(), output.as_deref(), &options.flags)
            .await
            .map(|_| EX_OK),

        #[cfg(feature = "index")]
        Command::Index { module, urls } => {
            commands::index::index(urls, module.as_deref(), &options.flags)
                .await
                .map(|_| EX_OK)
        },

        #[cfg(feature = "list")]
        Command::List {
            module,
            limit,
            output,
            urls,
        } => commands::list::list(
            urls,
            module.as_deref(),
            *limit,
            output.as_deref(),
            &options.flags,
        )
        .await
        .map(|_| EX_OK),

        #[cfg(feature = "read")]
        Command::Read { module, urls } => {
            commands::read::read(urls, module.as_deref(), &options.flags)
                .await
                .map(|_| EX_OK)
        },

        #[cfg(feature = "search")]
        Command::Search { module, prompt } => {
            commands::search::search(&prompt, module.as_deref(), &options.flags)
                .await
                .map(|_| EX_OK)
        },

        #[cfg(feature = "snap")]
        Command::Snap { urls } => commands::snap::snap(urls, &options.flags)
            .await
            .map(|_| EX_OK),

        Command::External(args) => {
            let cmd = External {
                is_debug: options.flags.debug,
                pipe_output: false,
            };

            cmd.execute(&args[0], &args[1..]).map(|result| result.code)
        },
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
