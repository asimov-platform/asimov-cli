// This is free and unencumbered software released into the public domain.

#![deny(unsafe_code)]

use asimov_cli::commands::{self, ExternalSubcommand, Help, HelpCmd};
use clientele::{
    StandardOptions, SubcommandsProvider,
    SysexitsError::{self, *},
    crates::clap::{Parser, Subcommand},
};
use color_print::ceprintln;
use std::ffi::OsString;

#[cfg(feature = "handle")]
use crate::commands::handle::HandleCommand;

#[cfg(feature = "message")]
use crate::commands::message::MessageCommand;

#[cfg(feature = "module")]
use crate::commands::module::ModuleCommand;

#[cfg(feature = "package")]
use crate::commands::package::PackageCommand;

#[cfg(feature = "protocol")]
use crate::commands::protocol::ProtocolCommand;

#[cfg(feature = "snapshot")]
use crate::commands::snapshot::SnapshotCommand;

/// ASIMOV Command-Line Interface (CLI)
#[derive(Debug, Parser)]
#[command(name = "asimov", long_about)]
#[command(allow_external_subcommands = true)]
#[command(arg_required_else_help = true)]
#[command(after_help = after_help())]
#[command(after_long_help = after_long_help())]
struct Options {
    #[clap(flatten)]
    flags: StandardOptions,

    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Prompt an LLM with text input
    #[cfg(feature = "ask")]
    Ask {
        #[clap(long, short = 'M')]
        module: Option<String>,

        #[clap(long, short = 'm')]
        model: Option<String>,

        input: Option<String>,
    },

    /// TBD
    #[cfg(feature = "describe")]
    #[clap(aliases = ["summarize", "tldr"])]
    Describe {
        #[clap(long, short = 'M')]
        module: Option<String>,

        /// The output format.
        #[arg(value_name = "FORMAT", short = 'o', long)]
        output: Option<String>,

        urls: Vec<String>,
    },

    /// Fetch knowledge from a URL, utilizing enabled modules
    #[cfg(feature = "fetch")]
    #[clap(aliases = ["extract", "get", "import", "parse"])]
    Fetch {
        /// Optionally choose the module instead of using module resolution.
        /// The module's manifest must declare support for the URL for the
        /// module to be used.
        #[clap(long, short = 'M')]
        module: Option<String>,

        /// The output format.
        #[arg(value_name = "FORMAT", short = 'o', long)]
        output: Option<String>,

        urls: Vec<String>,
    },

    /// Add, remove, and resolve handles and their associated endpoints
    #[cfg(feature = "handle")]
    #[clap(subcommand)]
    Handle(HandleCommand),

    /// TBD
    #[cfg(feature = "index")]
    Index {
        #[clap(long, short = 'm')]
        module: Option<String>,

        urls: Vec<String>,
    },

    /// Catalog knowledge from a URL, utilizing enabled modules
    #[cfg(feature = "list")]
    #[clap(aliases = ["dir", "ls"])]
    List {
        #[clap(long, short = 'M')]
        module: Option<String>,

        /// The maximum number of resources to list.
        #[arg(value_name = "COUNT", short = 'n', long)]
        limit: Option<usize>,

        /// The output format.
        #[arg(value_name = "FORMAT", short = 'o', long)]
        output: Option<String>,

        urls: Vec<String>,
    },

    /// Message other peers
    #[cfg(feature = "message")]
    #[clap(subcommand)]
    Message(MessageCommand),

    /// Manage modules, such as installing/enabling/disabling them
    #[cfg(feature = "module")]
    #[clap(subcommand)]
    Module(ModuleCommand),

    /// Package development commands
    #[cfg(feature = "package")]
    #[clap(subcommand)]
    Package(PackageCommand),

    /// Low-level protocol commands
    #[cfg(feature = "protocol")]
    #[clap(subcommand)]
    Protocol(ProtocolCommand),

    /// Read a resource specified by a URL, utilizing enabled modules
    #[cfg(feature = "read")]
    Read {
        #[clap(long, short = 'M')]
        module: Option<String>,

        urls: Vec<String>,
    },

    /// TBD
    #[cfg(feature = "search")]
    Search {
        #[clap(long, short = 'M')]
        module: Option<String>,

        prompt: String,
    },

    /// Save a snapshot for a URL, utilizing enabled modules
    #[cfg(feature = "snap")]
    Snap { urls: Vec<String> },

    /// Manage snapshots stored on disk
    #[cfg(feature = "snapshot")]
    #[clap(subcommand)]
    Snapshot(SnapshotCommand),

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
    let options = match Options::try_parse_from(&args) {
        Ok(options) => options,

        // VARIANT 1
        // this handles:
        // 1. `asimov`                    # DisplayHelpOnMissingArgumentOrSubcommand
        // 2. `asimov -h`                 # DisplayHelp
        // 3. `asimov --help`             # DisplayHelp
        // 4. `asimov help`               # DisplayHelp
        // 5. `asimov <known cmd> -h`     # DisplayHelp
        // 6. `asimov <known cmd> --help` # DisplayHelp
        // 7. `asimov help <known cmd>`   # DisplayHelp
        //
        // however it *doesn't* handle the cases:
        // 1. `asimov <unknown cmd> -h`
        // 2. `asimov <unknown cmd> --help`
        // 3. `asimov <unknown cmd> help`
        // 4. `asimov help <unknown cmd>`   # InvalidSubcommand
        //
        // where the unknown command is probably a subprogram such as:
        // - `asimov-module`
        // - `asimov-snapshot`
        //
        // note that cases 1, 2, and 3 are actually not clap errors and are handled by
        // `Command::External` which passes the `-h`/`--help`/`help` as args to the subprogram.
        //
        // only case 4 is an error but not a `ErrorKind::DisplayHelp`, it's handled in VARIANT 2,
        // below.
        Err(err)
            if err.kind() == clap::error::ErrorKind::DisplayHelp
                || err.kind()
                    == clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand =>
        {
            err.exit()
        },

        // VARIANT 2
        // situation:
        // - the error kind is ErrorKind::InvalidSubcommand
        // - the second arg (first user-provided arg) is `help`
        //
        // =>
        //
        // user desires help about a subprogram (`asimov-*`).
        Err(err)
            if err.kind() == clap::error::ErrorKind::InvalidSubcommand
                && args
                    .get(1)
                    .and_then(|arg| arg.to_str())
                    .is_some_and(|arg| arg == "help") =>
        {
            let debug =
                args.contains(&OsString::from("-d")) || args.contains(&OsString::from("--debug"));

            let cmd = HelpCmd { is_debug: debug };

            let Ok(args) = args
                .into_iter()
                .map(OsString::into_string)
                .collect::<Result<Vec<_>, _>>()
            else {
                return EX_USAGE;
            };

            // we know the first arg is binary itself, second arg is `help`, skip those.
            // then skip anything starting with `-`.
            let mut args = args
                .into_iter()
                .skip(2)
                .skip_while(|arg| arg.starts_with("-"));

            // next arg is subcommand
            let Some(cmd_name) = args.next() else {
                err.exit();
            };

            // collect rest as args to subcommand
            let args: Vec<String> = args.collect();

            // TODO: match color output. currently subprogram always outputs without colors

            // TODO: enable help from external program's subcommands (e.g. `asimov help module list`)

            let result = cmd.execute(&cmd_name, &args);
            if let Ok(result) = &result {
                if result.success {
                    let mut stdout = std::io::stdout().lock();
                    if std::io::copy(&mut result.output.as_slice(), &mut stdout).is_err() {
                        return EX_IOERR;
                    }
                } else {
                    eprintln!("asimov: {} doesn't provide help", cmd_name);

                    if debug {
                        eprintln!("asimov: status code - {}", result.code);

                        let mut stdout = std::io::stdout().lock();
                        if std::io::copy(&mut result.output.as_slice(), &mut stdout).is_err() {
                            return EX_IOERR;
                        }
                    }
                }
            }

            return result.map(|result| result.code).unwrap_or(EX_UNAVAILABLE);
        },

        // VARIANT 3
        // some other error, issue in provided args.
        // just let clap handle the error
        Err(err) => err.exit(),
    };
    let flags = &options.flags;

    asimov_module::init_tracing_subscriber(flags).expect("failed to initialize logging");

    // Print the version, if requested:
    if flags.version {
        println!("ASIMOV {}", env!("CARGO_PKG_VERSION"));
        return EX_OK;
    }

    // Print the license, if requested:
    if flags.license {
        print!("{}", include_str!("../UNLICENSE"));
        return EX_OK;
    }

    // Configure debug output:
    if flags.debug {
        //std::env::set_var("RUST_BACKTRACE", "1");
    }

    // From asimov-module-cli:
    asimov_registry::Registry::default()
        .create_file_tree()
        .await
        .inspect_err(|e| {
            tracing::debug!("failed to create module file tree: {e}");
        })
        .ok();

    // From asimov-snapshot-cli:
    if let Err(err) = std::fs::create_dir_all(asimov_env::paths::asimov_root().join("snapshots"))
        .map_err(|e| {
            ceprintln!("<s,r>error:</> failed to create snapshot directory: {e}");
            EX_IOERR
        })
    {
        return err;
    }

    // Execute the given command:
    use Command::*;
    let result = match options.command.unwrap() {
        #[cfg(feature = "ask")]
        Ask {
            module,
            model,
            input,
        } => {
            let input = if let Some(input) = input {
                input.clone()
            } else {
                use std::io::Read;
                let mut buf = String::new();
                if std::io::stdin().read_to_string(&mut buf).is_err() {
                    return EX_IOERR;
                };
                buf
            };
            commands::ask::ask(input, module, model, flags)
                .await
                .map(|_| EX_OK)
        },

        #[cfg(feature = "describe")]
        Describe {
            module,
            output,
            urls,
        } => commands::describe::describe(urls, module, output, flags)
            .await
            .map(|_| EX_OK),

        #[cfg(feature = "fetch")]
        Fetch {
            module,
            output,
            urls,
        } => commands::fetch::fetch(urls, module, output, flags)
            .await
            .map(|_| EX_OK),

        #[cfg(feature = "handle")]
        Handle(command) => command.run(flags).await.map_err(sysexits).map(|_| EX_OK),

        #[cfg(feature = "index")]
        Index { module, urls } => commands::index::index(urls, module, flags)
            .await
            .map(|_| EX_OK),

        #[cfg(feature = "list")]
        List {
            module,
            limit,
            output,
            urls,
        } => commands::list::list(urls, module, limit, output, flags)
            .await
            .map(|_| EX_OK),

        #[cfg(feature = "message")]
        Message(command) => command.run(flags).await.map_err(sysexits).map(|_| EX_OK),

        #[cfg(feature = "module")]
        Module(command) => command.run(flags).await.map_err(sysexits).map(|_| EX_OK),

        #[cfg(feature = "package")]
        Package(command) => command.run(flags).await.map_err(sysexits).map(|_| EX_OK),

        #[cfg(feature = "protocol")]
        Protocol(command) => command.run(flags).await.map_err(sysexits).map(|_| EX_OK),

        #[cfg(feature = "read")]
        Read { module, urls } => commands::read::read(urls, module, flags)
            .await
            .map(|_| EX_OK),

        #[cfg(feature = "search")]
        Search { module, prompt } => commands::search::search(prompt, module, flags)
            .await
            .map(|_| EX_OK),

        #[cfg(feature = "snap")]
        Snap { urls } => commands::snap::snap(urls, flags).await.map(|_| EX_OK),

        #[cfg(feature = "snapshot")]
        Snapshot(command) => command.run(flags).await.map(|_| EX_OK),

        External(args) => {
            let cmd = ExternalSubcommand {
                is_debug: flags.debug,
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

fn after_long_help() -> String {
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

    help
}

pub fn after_help() -> String {
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

    help
}

// `From<Box<dyn Error>> for SysexitsError` discards the original code,
// mapping everything to EX_SOFTWARE; recover it by downcasting instead.
fn sysexits(err: Box<dyn std::error::Error>) -> SysexitsError {
    err.downcast_ref::<SysexitsError>()
        .copied()
        .unwrap_or(EX_SOFTWARE)
}
