// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::{string::String, vec::Vec};
use clientele::{StandardOptions, crates::clap::Subcommand};

/// Commands for managing tracked resources.
#[derive(Debug, Subcommand)]
pub enum ResourceCommand {
    /// List the set of tracked resources
    #[clap(alias = "ls")]
    List {},

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
            List {} => list(flags).await,
            Track { urls } => track(urls, flags).await,
            Untrack { urls } => untrack(urls, flags).await,
            Search { query } => search(query, flags).await,
        }
    }
}

mod list;
pub use list::*;

mod search;
pub use search::*;

mod track;
pub use track::*;

mod untrack;
pub use untrack::*;
