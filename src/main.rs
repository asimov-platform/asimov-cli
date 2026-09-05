// This is free and unencumbered software released into the public domain.

#![deny(unsafe_code)]

use asimov_cli::{
    BoxError,
    commands::{self, ExternalSubcommand, Help, HelpCmd},
};
use clientele::{
    ColorChoiceExt, StandardOptions, SubcommandsProvider,
    SysexitsError::{self, *},
    crates::clap::{CommandFactory, FromArgMatches, Parser, Subcommand},
    strip_ansi,
};
use color_print::ceprintln;
use std::ffi::OsString;

#[cfg(feature = "module")]
use crate::commands::module::ModuleCommand;

#[cfg(feature = "proxy")]
use crate::commands::proxy::ProxyCommand;

#[cfg(feature = "source")]
use crate::commands::source::SourceCommand;

// #[cfg(feature = "handle")]
// use crate::commands::unstable::handle::HandleCommand;

// #[cfg(feature = "message")]
// use crate::commands::unstable::message::MessageCommand;

// #[cfg(feature = "package")]
// use crate::commands::unstable::package::PackageCommand;

// #[cfg(feature = "protocol")]
// use crate::commands::unstable::protocol::ProtocolCommand;

/// ASIMOV Command-Line Interface (CLI)
#[derive(Debug, Parser)]
#[command(name = "asimov", long_about)]
#[command(allow_external_subcommands = true)]
#[command(arg_required_else_help = true)]
#[command(styles = clientele::HELP_STYLES)]
struct Options {
    #[clap(flatten)]
    flags: StandardOptions,

    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Manage modules, installing/enabling/disabling them
    #[cfg(feature = "module")]
    #[clap(subcommand)]
    Module(ModuleCommand),

    /// Run a local OpenAI-compatible proxy endpoint
    #[cfg(feature = "proxy")]
    Proxy {
        #[clap(subcommand)]
        command: Option<ProxyCommand>,

        #[clap(flatten)]
        args: commands::proxy::ProxyServeArgs,
    },

    /// Manage data sources, fetching/listing/snapshotting them
    #[cfg(feature = "source")]
    Source {
        #[clap(subcommand)]
        command: Option<SourceCommand>,

        #[clap(flatten)]
        args: commands::source::SourceFetchArgs,
    },

    #[cfg(feature = "unstable")]
    #[clap(flatten)]
    Unstable(commands::unstable::UnstableCommand),

    #[clap(external_subcommand)]
    External(Vec<String>),
}

#[tokio::main]
pub async fn main() -> SysexitsError {
    // Load environment variables from `.env`:
    clientele::dotenv().ok();

    // Expand wildcards and @argfiles:
    let Ok(mut args) = clientele::args_os() else {
        return EX_USAGE;
    };

    // Resolve command aliases (e.g. `asimov fetch` -> `asimov source fetch`):
    asimov_cli::aliases::resolve(&mut args);

    // Determine the color output mode ahead of parsing, so that clap's own
    // help/usage/error rendering honors `--color` (the default is "auto"):
    let color = clientele::color_choice(&args);
    let use_color = color.to_bool();

    // Parse command-line options:
    let options = Options::command()
        .color(color)
        .help_template(help_template(use_color))
        .after_help(after_help(use_color))
        .after_long_help(after_long_help(use_color))
        .try_get_matches_from(&args)
        .and_then(|mut matches| {
            Options::from_arg_matches_mut(&mut matches)
                .map_err(|err| err.format(&mut Options::command().color(color)))
        });
    let options = match options {
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

    // Print help if no command was given (e.g. `asimov --color=never`),
    // mirroring the behavior of a bare `asimov` invocation:
    let Some(command) = options.command else {
        Options::command()
            .color(color)
            .help_template(help_template(use_color))
            .after_help(after_help(use_color))
            .print_help()
            .ok();
        return EX_USAGE;
    };

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
    let result = match command {
        #[cfg(feature = "module")]
        Module(command) => command.run(flags).await.map_err(sysexits).map(|_| EX_OK),

        #[cfg(feature = "proxy")]
        Proxy { command, args } => command
            .unwrap_or(ProxyCommand::Serve { args })
            .run(flags)
            .await
            .map_err(sysexits)
            .map(|_| EX_OK),

        #[cfg(feature = "source")]
        Source { command, args } => command
            .unwrap_or(SourceCommand::Fetch { args })
            .run(flags)
            .await
            .map_err(sysexits)
            .map(|_| EX_OK),

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

/// Builds the help template, inserting an "Aliases" section between the
/// default "Commands" and "Options" sections.
fn help_template(color: bool) -> String {
    let aliases = if color {
        aliases_help()
    } else {
        strip_ansi(&aliases_help())
    };
    let commands_heading = color_print::cstr!("<y>Commands:</y>");
    let options_heading = color_print::cstr!("<y>Options:</y>");
    let (commands_heading, options_heading) = if color {
        (commands_heading.into(), options_heading.into())
    } else {
        (strip_ansi(commands_heading), strip_ansi(options_heading))
    };
    format!(
        "{{before-help}}{{about-with-newline}}\n{{usage-heading}} {{usage}}\n\n{commands_heading}\n{{subcommands}}\n\n{aliases}\n{options_heading}\n{{options}}{{after-help}}"
    )
}

/// Renders the "Aliases" help section listing the hardcoded command aliases.
fn aliases_help() -> String {
    let mut help = String::new();
    help.push_str(color_print::cstr!("<y>Aliases:</y>\n"));

    let width = asimov_cli::aliases::ALIASES
        .iter()
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or(0);

    for (name, expansion) in asimov_cli::aliases::ALIASES {
        help.push_str(&color_print::cformat!(
            "  <s>{:width$}</s>  asimov {}\n",
            name,
            expansion.join(" "),
        ));
    }

    help
}

fn after_long_help(color: bool) -> String {
    let mut help = String::new();
    let cmds = Help.execute();
    for (i, cmd) in cmds.iter().enumerate() {
        if i == 0 {
            help.push_str(color_print::cstr!("<s><u>Commands:</u></s>\n"));
        }
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

    if color { help } else { strip_ansi(&help) }
}

pub fn after_help(color: bool) -> String {
    let mut help = String::new();
    let commands = SubcommandsProvider::collect("asimov-", 1);
    for (i, cmd) in commands.iter().enumerate() {
        if i == 0 {
            help.push_str(color_print::cstr!("<s><u>Commands:</u></s>\n"));
        }
        if i > 0 {
            help.push('\n');
        }

        help.push_str(&color_print::cformat!(
            "\t<dim>$</dim> <s>asimov {}</s> [OPTIONS] [COMMAND]",
            cmd.name,
        ));
    }

    if color { help } else { strip_ansi(&help) }
}

// `From<Box<dyn Error>> for SysexitsError` discards the original code,
// mapping everything to EX_SOFTWARE; recover it by downcasting instead.
fn sysexits(err: BoxError) -> SysexitsError {
    err.downcast_ref::<SysexitsError>()
        .copied()
        .unwrap_or(EX_SOFTWARE)
}
