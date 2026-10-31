// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::{string::String, vec::Vec};
use asimov_directory::{
    UrisFile,
    fs::{ResourcesDirectory, StateDirectory},
};
use asimov_module::ModuleName;
use clientele::{
    StandardOptions,
    SysexitsError::*,
    crates::clap::{Args, Subcommand},
};
use std::io::{self, Write};

/// Concurrency control for commands that process each URL as a separate job.
#[derive(Args, Clone, Copy, Debug, Default)]
pub struct JobsOptions {
    /// The maximum number of URLs to process at once [default: number of CPUs].
    #[arg(short = 'j', long, value_name = "COUNT")]
    pub jobs: Option<core::num::NonZeroUsize>,
}

impl JobsOptions {
    /// Returns the requested job count, or else the number of CPUs.
    pub fn count(&self) -> usize {
        self.jobs
            .or_else(|| std::thread::available_parallelism().ok())
            .map_or(1, core::num::NonZeroUsize::get)
    }
}

/// Commands for managing tracked resources.
#[derive(Debug, Subcommand)]
pub enum ResourceCommand {
    /// Fetch a resource using enabled modules, or ASIMOV_RELAY_URL as fallback.
    #[clap(aliases = ["extract", "get", "import", "ingest", "parse"])]
    Fetch {
        #[clap(flatten)]
        args: ResourceFetchArgs,
    },

    /// List tracked resources or resources at the given URLs
    ///
    /// With URLs, uses enabled modules, or ASIMOV_RELAY_URL as fallback.
    /// Without URLs, prints tracked URLs in stored order, one per line.
    #[clap(aliases = ["dir", "ls"])]
    List {
        #[clap(flatten)]
        args: ResourceListArgs,
    },

    /// Read a resource specified by a URL, utilizing enabled modules
    Read {
        #[clap(long, short = 'M')]
        module: Option<ModuleName>,

        #[arg(required = true)]
        urls: Vec<String>,
    },

    /// Manage snapshots stored on disk
    #[cfg(feature = "unstable")]
    Snap {
        #[clap(subcommand)]
        command: Option<SnapCommand>,

        #[clap(flatten)]
        args: SnapSaveArgs,
    },

    /// Track the given resource URLs
    #[clap(alias = "add")]
    Track {
        /// The resource URLs to track
        #[arg(required = true)]
        urls: Vec<String>,
    },

    /// Untrack the given resource URLs
    #[clap(aliases = ["remove", "rm"])]
    Untrack {
        /// The resource URLs to untrack
        #[arg(required = true)]
        urls: Vec<String>,
    },

    /// Search all tracked resources
    #[cfg(feature = "unstable")]
    #[clap(alias = "find")]
    Search {
        /// The terms to search for
        #[arg(required = true)]
        query: Vec<String>,
    },
}

impl ResourceCommand {
    /// Dispatches a resource command to its handler.
    pub async fn run(self, flags: &StandardOptions) -> Result<(), BoxError> {
        use ResourceCommand::*;
        match self {
            Fetch { args } => fetch(args, flags).await,
            List { args } => list(args, flags).await,
            Read { module, urls } => read(urls, module, flags).await,
            #[cfg(feature = "unstable")]
            Snap { command, args } => {
                command
                    .unwrap_or(SnapCommand::Save { args })
                    .run(flags)
                    .await
            },
            Track { urls } => track(urls, flags).await,
            Untrack { urls } => untrack(urls, flags).await,
            #[cfg(feature = "unstable")]
            Search { query } => search(query, flags).await,
        }
    }
}

mod fetch;
pub use fetch::*;

mod list;
pub use list::*;

mod read;
pub use read::*;

#[cfg(feature = "unstable")]
mod search;
#[cfg(feature = "unstable")]
pub use search::*;

#[cfg(feature = "unstable")]
pub mod snap;
#[cfg(feature = "unstable")]
pub use snap::{SnapCommand, SnapSaveArgs};

mod track;
pub use track::*;

mod untrack;
pub use untrack::*;

fn resources() -> Result<ResourcesDirectory, BoxError> {
    let state = StateDirectory::from_env().map_err(|e| report("open state directory", e))?;
    state
        .resources()
        .map_err(|e| report("open resource storage", e))
}

fn parse_uris(uris: Vec<String>) -> Result<UrisFile, BoxError> {
    UrisFile::new(uris).map_err(|error| {
        report(
            "validate resource IRIs",
            io::Error::new(io::ErrorKind::InvalidInput, error),
        )
    })
}

fn report(operation: &str, error: io::Error) -> BoxError {
    if crate::shared::is_stdout_closed(&error) {
        return error.into();
    }
    let _ = writeln!(
        io::stderr().lock(),
        "asimov: failed to {operation}: {error}"
    );
    match error.kind() {
        io::ErrorKind::InvalidInput | io::ErrorKind::InvalidFilename => EX_USAGE,
        io::ErrorKind::InvalidData => EX_DATAERR,
        io::ErrorKind::NotFound => EX_NOINPUT,
        io::ErrorKind::AlreadyExists => EX_CANTCREAT,
        io::ErrorKind::PermissionDenied => EX_NOPERM,
        _ => EX_IOERR,
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clientele::crates::clap::Parser;

    #[derive(Parser)]
    struct Command {
        #[command(subcommand)]
        command: ResourceCommand,
    }

    #[test]
    fn list_parses_shared_options() {
        let Command {
            command:
                ResourceCommand::List {
                    args:
                        ResourceListArgs {
                            cache,
                            timing,
                            filtering,
                            urls,
                            ..
                        },
                },
        } = Command::try_parse_from([
            "test",
            "ls",
            "--max-age",
            "1h",
            "--deadline",
            "0s",
            "--jev",
            "The name is Ukrainian",
            "--json",
            "@id,name",
            "--jq",
            ".name",
            "https://example.com/",
        ])
        .unwrap()
        else {
            panic!("expected resource list");
        };
        assert_eq!(cache.max_age_option().as_deref(), Some("--max-age=1h"));
        assert_eq!(timing.deadline_option().as_deref(), Some("--deadline=0s"));
        assert_eq!(filtering.jev.as_deref(), Some("The name is Ukrainian"));
        assert_eq!(filtering.json.unwrap(), ["@id", "name"]);
        assert_eq!(filtering.jq.as_deref(), Some(".name"));
        assert_eq!(urls, ["https://example.com/"]);
    }
}
