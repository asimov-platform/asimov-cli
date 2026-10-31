// This is free and unencumbered software released into the public domain.

#![deny(unsafe_code)]

#[cfg(any(
    test,
    feature = "module",
    all(feature = "proxy", feature = "unstable"),
    feature = "remote",
    feature = "resource",
    feature = "telemetry"
))]
use asimov_cli::BoxError;
#[cfg(any(
    feature = "module",
    feature = "remote",
    feature = "resource",
    feature = "telemetry",
    feature = "unstable"
))]
use asimov_cli::commands;
use asimov_cli::{
    commands::{ExternalSubcommand, Help, HelpCmd},
    shared::telemetry::{self, CommandMetadata},
};
use asimov_keyring::{Keyring, KeyringError};
use clientele::{
    ColorChoiceExt, StandardOptions, SubcommandsProvider,
    SysexitsError::*,
    crates::clap::{CommandFactory, FromArgMatches, Parser, Subcommand},
    strip_ansi,
};
use color_print::ceprintln;
use std::ffi::OsString;

#[cfg(feature = "telemetry")]
use crate::commands::configure::{ConfigureCommand, TelemetryCommand};

#[cfg(feature = "module")]
use crate::commands::module::ModuleCommand;

#[cfg(all(feature = "proxy", feature = "unstable"))]
use crate::commands::proxy::ProxyCommand;

#[cfg(feature = "remote")]
use crate::commands::remote::RemoteCommand;

#[cfg(feature = "resource")]
use crate::commands::resource::ResourceCommand;

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
    /// Configure the CLI itself
    #[cfg(feature = "telemetry")]
    #[clap(name = "config", alias = "configure")]
    #[clap(subcommand)]
    Configure(ConfigureCommand),

    /// Manage modules, installing/enabling/disabling them
    #[cfg(feature = "module")]
    #[clap(subcommand)]
    Module(ModuleCommand),

    /// Run a local OpenAI-compatible proxy endpoint
    #[cfg(all(feature = "proxy", feature = "unstable"))]
    Proxy {
        #[clap(subcommand)]
        command: Option<ProxyCommand>,

        #[clap(flatten)]
        args: commands::proxy::ProxyServeArgs,
    },

    /// Manage the set of tracked remotes
    #[cfg(feature = "remote")]
    #[clap(subcommand)]
    Remote(RemoteCommand),

    /// Manage the set of tracked resources
    #[cfg(feature = "resource")]
    #[clap(alias = "source")]
    #[clap(subcommand)]
    Resource(ResourceCommand),

    #[cfg(feature = "unstable")]
    #[clap(flatten)]
    Unstable(commands::unstable::UnstableCommand),

    #[clap(external_subcommand)]
    External(Vec<OsString>),
}

#[tokio::main]
pub async fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(run().await)
}

/// Runs the given command line, returning the process exit code.
///
/// Built-in commands exit with `sysexits(3)` codes, but external commands'
/// exit codes are passed through as is.
async fn run() -> u8 {
    // Load `$ASIMOV_ROOT/.env` before the nearest `.env`:
    clientele::crates::dotenvy::from_path(asimov_env::paths::asimov_root().join(".env")).ok();
    clientele::dotenv().ok();

    // Expand wildcards and @argfiles:
    let Ok(mut args) = clientele::args_os() else {
        return EX_USAGE.as_u8();
    };

    // Resolve command aliases (e.g. `asimov fetch` -> `asimov resource fetch`):
    asimov_cli::aliases::resolve(&mut args);
    let help_position =
        asimov_cli::aliases::command_position(&args).filter(|&index| args[index] == "help");

    // Determine the color output mode ahead of parsing, so that clap's own
    // help/usage/error rendering honors `--color` (the default is "auto"):
    let color = clientele::color_choice(&args);
    let use_color = color.to_bool();

    // Parse command-line options:
    let parser = Options::command()
        .color(color)
        .help_template(help_template(use_color))
        .after_help(after_help(use_color));
    let matches = parser.clone().try_get_matches_from(&args);
    let mut matches = match matches {
        Ok(matches) => matches,

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
            // External summaries require subprocesses; collect them only for
            // root long help, after Clap has confirmed this is a help request.
            if err.kind() == clap::error::ErrorKind::DisplayHelp
                && root_long_help_requested(&args)
                && let Err(help) = parser
                    .after_long_help(after_long_help(use_color))
                    .try_get_matches_from(&args)
            {
                help.exit();
            }
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
                && help_position.is_some() =>
        {
            let debug =
                args.contains(&OsString::from("-d")) || args.contains(&OsString::from("--debug"));

            let cmd = HelpCmd { is_debug: debug };

            let Ok(args) = args
                .into_iter()
                .map(OsString::into_string)
                .collect::<Result<Vec<_>, _>>()
            else {
                return EX_USAGE.as_u8();
            };

            // Skip the program, leading global options, and the help command.
            let mut args = args
                .into_iter()
                .skip(help_position.unwrap_or(1) + 1)
                .skip_while(|arg| arg.starts_with("-"));

            // next arg is subcommand
            let Some(cmd_name) = args.next() else {
                err.exit();
            };

            // collect rest as args to subcommand
            let args: Vec<String> = args.collect();

            // TODO: match color output. currently subprogram always outputs without colors

            let result = cmd.execute(&cmd_name, &args);
            if let Ok(result) = &result {
                if result.success {
                    let mut stdout = std::io::stdout().lock();
                    if std::io::copy(&mut result.output.as_slice(), &mut stdout).is_err() {
                        return EX_IOERR.as_u8();
                    }
                } else {
                    eprintln!("asimov: {} doesn't provide help", cmd_name);

                    if debug {
                        eprintln!("asimov: status code - {}", result.code);

                        let mut stdout = std::io::stdout().lock();
                        if std::io::copy(&mut result.output.as_slice(), &mut stdout).is_err() {
                            return EX_IOERR.as_u8();
                        }
                    }
                }
            }

            return result
                .map(|result| result.code)
                .unwrap_or(EX_UNAVAILABLE)
                .as_u8();
        },

        // VARIANT 3
        // some other error, issue in provided args.
        // just let clap handle the error
        Err(err) => err.exit(),
    };
    let metadata = CommandMetadata::from_matches::<Options>(&matches);
    let options = match Options::from_arg_matches_mut(&mut matches) {
        Ok(options) => options,
        Err(err) => err.format(&mut Options::command().color(color)).exit(),
    };
    let flags = &options.flags;

    asimov_module::init_tracing_subscriber(flags).expect("failed to initialize logging");

    // Print the version, if requested:
    if flags.version {
        println!("ASIMOV {}", env!("CARGO_PKG_VERSION"));
        return EX_OK.as_u8();
    }

    // Print the license, if requested:
    if flags.license {
        print!("{}", include_str!("../UNLICENSE"));
        return EX_OK.as_u8();
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
        return EX_USAGE.as_u8();
    };

    // Read the local public identity, initializing keys only if it is absent.
    if let Err(error) = Keyring::my_public_key() {
        ceprintln!("<s,r>error:</> failed to initialize local identity: {error}");
        return match error {
            KeyringError::IoError(_) | KeyringError::RekeyRollbackFailed { .. } => EX_IOERR,
            KeyringError::KeyError(_) | KeyringError::CorruptSecret { .. } => EX_DATAERR,
            KeyringError::KeyringError(_) => EX_UNAVAILABLE,
            KeyringError::UserNotFound => EX_NOUSER,
            KeyringError::InvalidUser(_) => EX_CONFIG,
            KeyringError::LockPoisoned => EX_SOFTWARE,
        }
        .as_u8();
    }

    match command {
        // Starting up could flush the very events this command is about to discard.
        #[cfg(feature = "telemetry")]
        Command::Configure(ConfigureCommand::Telemetry(TelemetryCommand::Disable {})) => {},
        _ => telemetry::initialize(),
    }
    let telemetry = metadata.start();

    let result = async {
        let needs_modules = match &command {
            #[cfg(feature = "module")]
            Command::Module(_) => true,
            #[cfg(feature = "resource")]
            Command::Resource(ResourceCommand::Fetch { .. } | ResourceCommand::Read { .. }) => true,
            #[cfg(feature = "resource")]
            Command::Resource(ResourceCommand::List { args }) => !args.urls.is_empty(),
            #[cfg(all(feature = "resource", feature = "unstable"))]
            Command::Resource(ResourceCommand::Snap { .. }) => true,
            _ => false,
        };
        if needs_modules {
            asimov_registry::Registry::default()
                .create_file_tree()
                .await
                .inspect_err(|e| {
                    tracing::debug!("failed to create module file tree: {e}");
                })
                .ok();
        }

        #[cfg(all(feature = "resource", feature = "unstable"))]
        if matches!(&command, Command::Resource(ResourceCommand::Snap { .. }))
            && let Err(err) = std::fs::create_dir_all(
                asimov_env::paths::asimov_root().join("snapshots"),
            )
            .map_err(|e| {
                ceprintln!("<s,r>error:</> failed to create snapshot directory: {e}");
                EX_IOERR
            })
        {
            return err.as_u8();
        }

        // Execute the given command:
        use Command::*;
        match command {
            #[cfg(feature = "telemetry")]
            Configure(command) => exit_code(command.run(flags).await),

            #[cfg(feature = "module")]
            Module(command) => exit_code(command.run(flags).await),

            #[cfg(all(feature = "proxy", feature = "unstable"))]
            Proxy { command, args } => exit_code(
                command
                    .unwrap_or(ProxyCommand::Serve { args })
                    .run(flags)
                    .await,
            ),

            #[cfg(feature = "remote")]
            Remote(command) => exit_code(command.run(flags).await),

            #[cfg(feature = "resource")]
            Resource(command) => exit_code(command.run(flags).await),

            External(args) => {
                let cmd = ExternalSubcommand {
                    is_debug: flags.debug,
                    pipe_output: false,
                };
                let Some(name) = args.first().and_then(|name| name.to_str()) else {
                    eprintln!("asimov: external command name must be valid UTF-8");
                    return EX_USAGE.as_u8();
                };
                match cmd.execute_os(name, &args[1..]) {
                    Ok(result) => result.code,
                    Err(err) => err.as_u8(),
                }
            },
        }
    }
    .await;

    telemetry.finish(result.into());
    telemetry::shutdown();

    result
}

/// Builds the help template, inserting an "Aliases" section between the
/// default "Commands" and "Options" sections.
fn help_template(color: bool) -> String {
    let aliases = if color {
        aliases_help()
    } else {
        strip_ansi(aliases_help())
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
    if asimov_cli::aliases::ALIASES.is_empty() {
        return String::new();
    }
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

fn root_long_help_requested(args: &[OsString]) -> bool {
    let mut args = args.iter().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--" | "-h") => return false,
            Some("--color") => {
                args.next();
            },
            Some("--help") => return true,
            Some("help") => return args.next().is_none(),
            Some(arg) if arg.starts_with('-') => {},
            _ => return false,
        }
    }
    false
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

/// Maps a built-in command's result to its `sysexits(3)` exit code.
#[cfg(any(
    feature = "module",
    all(feature = "proxy", feature = "unstable"),
    feature = "remote",
    feature = "resource",
    feature = "telemetry"
))]
fn exit_code<T>(result: Result<T, BoxError>) -> u8 {
    result.map_or_else(sysexits, |_| EX_OK).as_u8()
}

// `From<Box<dyn Error>> for SysexitsError` discards the original code,
// mapping everything to EX_SOFTWARE; recover it by downcasting instead.
#[cfg(any(
    test,
    feature = "module",
    all(feature = "proxy", feature = "unstable"),
    feature = "remote",
    feature = "resource",
    feature = "telemetry"
))]
fn sysexits(err: BoxError) -> clientele::SysexitsError {
    if let Some(code) = err.downcast_ref::<clientele::SysexitsError>() {
        return *code;
    }
    use std::io::Write;
    let _ = writeln!(std::io::stderr().lock(), "asimov: {err}");
    if let Some(error) = err.downcast_ref::<std::io::Error>() {
        match error.kind() {
            std::io::ErrorKind::InvalidData => EX_DATAERR,
            std::io::ErrorKind::PermissionDenied => EX_NOPERM,
            _ => EX_IOERR,
        }
    } else {
        EX_SOFTWARE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "resource")]
    fn parse(
        args: impl IntoIterator<Item = impl Into<OsString>>,
    ) -> core::result::Result<Options, clap::Error> {
        let mut args = args.into_iter().map(Into::into).collect();
        asimov_cli::aliases::resolve(&mut args);
        Options::try_parse_from(args)
    }

    #[test]
    #[cfg(feature = "resource")]
    fn resource_fetch_and_read_require_urls() {
        let url = "https://example.com/";
        for operation in [
            "fetch", "extract", "get", "import", "ingest", "parse", "read",
        ] {
            for group in ["resource", "source"] {
                let options = parse(["asimov", group, operation, url]).unwrap();
                if operation == "read" {
                    assert!(matches!(
                        options.command,
                        Some(Command::Resource(ResourceCommand::Read { .. }))
                    ));
                } else {
                    assert!(matches!(
                        options.command,
                        Some(Command::Resource(ResourceCommand::Fetch { .. }))
                    ));
                }
                let error = parse(["asimov", group, operation]).unwrap_err();
                assert_eq!(
                    error.kind(),
                    clap::error::ErrorKind::MissingRequiredArgument
                );
            }
        }
    }

    #[test]
    #[cfg(feature = "resource")]
    fn resource_listing_accepts_optional_urls() {
        for operation in ["list", "ls", "dir"] {
            for urls in [vec![], vec!["https://example.com/", "https://example.org/"]] {
                for group in ["resource", "source"] {
                    let options = parse(
                        ["asimov", group, operation]
                            .into_iter()
                            .chain(urls.iter().copied()),
                    )
                    .unwrap();
                    let Some(Command::Resource(ResourceCommand::List { args })) = options.command
                    else {
                        panic!("expected resource list");
                    };
                    assert_eq!(args.urls, urls);
                }
            }
        }
    }

    #[test]
    #[cfg(feature = "resource")]
    fn resource_snap_requires_unstable() {
        for group in ["resource", "source"] {
            let result = parse(["asimov", group, "snap", "compact"]);
            #[cfg(feature = "unstable")]
            assert!(matches!(
                result.unwrap().command,
                Some(Command::Resource(ResourceCommand::Snap {
                    command: Some(commands::resource::SnapCommand::Compact { .. }),
                    ..
                }))
            ));
            #[cfg(not(feature = "unstable"))]
            assert_eq!(
                result.unwrap_err().kind(),
                clap::error::ErrorKind::InvalidSubcommand
            );
        }
    }

    #[test]
    fn preserves_explicit_exit_codes_and_classifies_io_failures() {
        assert_eq!(sysexits(EX_CONFIG.into()), EX_CONFIG);
        for (kind, expected) in [
            (std::io::ErrorKind::InvalidData, EX_DATAERR),
            (std::io::ErrorKind::PermissionDenied, EX_NOPERM),
            (std::io::ErrorKind::BrokenPipe, EX_IOERR),
            (std::io::ErrorKind::Other, EX_IOERR),
        ] {
            assert_eq!(
                sysexits(std::io::Error::new(kind, "test I/O failure").into()),
                expected
            );
        }
        assert_eq!(sysexits("test unclassified failure".into()), EX_SOFTWARE);
    }
}
