// This is free and unencumbered software released into the public domain.

#![deny(unsafe_code)]

mod feature;

use clientele::{
    crates::clap::{CommandFactory, Parser, Subcommand},
    exit, StandardOptions,
    SysexitsError::*,
};
use std::{
    collections::BTreeSet,
    env::consts::EXE_SUFFIX,
    path::{Path, PathBuf},
    process::Stdio,
};

#[cfg(unix)]
use std::fs::Metadata;

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

    // Print the help message, if requested:
    if options.help {
        let mut help = String::new();
        help.push_str(color_print::cstr!("<s><u>Commands:</u></s>\n"));
        let subcommands = find_external_subcommands("asimov-").unwrap_or_default();
        for (i, subcommand_name) in subcommands.iter().enumerate() {
            if i > 0 {
                help.push('\n');
            }
            help.push_str(&color_print::cformat!(
                "\t<dim>$</dim> <s>asimov {}</s> [OPTIONS] [COMMAND]",
                subcommand_name,
            ));
        }
        Options::command()
            .after_long_help(help)
            .print_long_help()
            .unwrap();
        exit(EX_OK);
    }

    // Configure debug output:
    if options.flags.debug {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    // Locate the given subcommand:
    let Command::External(command) = &options.command.unwrap();
    let Some(command_path) = find_external_subcommand("asimov-", &command[0]) else {
        eprintln!(
            "{}: command not found: {}{}{}",
            "asimov", "asimov-", command[0], EXE_SUFFIX
        );
        exit(EX_UNAVAILABLE);
    };

    // Execute the given subcommand:
    let status = std::process::Command::new(command_path)
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

fn find_external_subcommands(prefix: &str) -> std::io::Result<BTreeSet<String>> {
    Ok(find_external_commands(prefix)?
        .iter()
        .filter_map(|path| {
            path.with_extension("").file_name().map(|name| {
                name.to_string_lossy()
                    .trim_start_matches(prefix)
                    .to_string()
            })
        })
        .collect())
}

fn find_external_commands(prefix: &str) -> std::io::Result<Vec<PathBuf>> {
    let prefix = prefix.as_bytes();
    let mut result = vec![];
    let Some(paths) = std::env::var_os("PATH") else {
        return Ok(result);
    };
    for path in std::env::split_paths(&paths) {
        let Ok(dir) = std::fs::read_dir(path) else {
            continue;
        };

        #[cfg(windows)]
        {
            use std::env::consts::EXE_EXTENSION;
            use std::os::windows::fs::MetadataExt;
            const FILE_ATTRIBUTE_HIDDEN: u32 = 0x00000002;

            for entry in dir {
                let entry = entry?;
                let entry_name = entry.file_name();
                let entry_bytes = entry_name.as_encoded_bytes();
                let metadata = entry.metadata()?;
                let is_hidden = metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0;
                let path = entry.path();
                let ext = path.extension().and_then(|ext| ext.to_str());
                if is_hidden || ext != Some(EXE_EXTENSION) || !entry_bytes.starts_with(prefix) {
                    continue;
                }

                result.push(path);
            }
        }

        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;

            for entry in dir {
                let entry = entry?;
                let entry_name = entry.file_name();
                let entry_bytes = entry_name.as_bytes();
                if entry_bytes.starts_with(b".")
                    || entry_bytes.ends_with(b"~")
                    || !entry_bytes.starts_with(prefix)
                    || !is_executable_metadata(entry.metadata()?)
                {
                    continue; // skip hidden, backup, and non-executable files
                }
                result.push(entry.path());
            }
        }
    }
    Ok(result)
}

fn find_external_subcommand(prefix: &str, subcommand: &str) -> Option<PathBuf> {
    find_external_command(&format!("{}{}", prefix, subcommand))
}

fn find_external_command(command: &str) -> Option<PathBuf> {
    let command_exe = format!("{}{}", command, EXE_SUFFIX);
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|path| path.join(&command_exe))
            .find(|path| is_executable_path(path))
    })
}

#[cfg(windows)]
fn is_executable_path(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_file()
}

#[cfg(unix)]
fn is_executable_path(path: impl AsRef<Path>) -> bool {
    std::fs::metadata(path)
        .map(is_executable_metadata)
        .unwrap_or(false)
}

#[cfg(unix)]
fn is_executable_metadata(metadata: Metadata) -> bool {
    use std::os::unix::prelude::*;
    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
}
