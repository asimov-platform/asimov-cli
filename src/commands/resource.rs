// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::{string::String, vec::Vec};
use asimov_module::ModuleName;
use clientele::{StandardOptions, crates::clap::Subcommand};

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
    /// Listing tracked resources without URLs is not yet implemented.
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
            Track { urls } => track(urls, flags).await,
            Untrack { urls } => untrack(urls, flags).await,
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

mod search;
pub use search::*;

mod track;
pub use track::*;

mod untrack;
pub use untrack::*;

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
