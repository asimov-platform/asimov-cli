// This is free and unencumbered software released into the public domain.

use clientele::{StandardOptions, SysexitsError, crates::clap::Subcommand};
use std::{string::String, vec::Vec};

#[derive(Debug, Subcommand)]
pub enum SnapshotCommand {
    /// Create snapshots for URL(s)
    #[clap(external_subcommand)]
    Snapshot(Vec<String>),

    /// List snapshots
    List,

    /// Show log for a URL
    Log {
        /// URL to show log for
        url: String,
    },

    /// Compact snapshots for a URL
    Compact {
        /// URL(s) to compact snapshots for
        urls: Vec<String>,
    },
}

impl SnapshotCommand {
    pub async fn run(&self, flags: &StandardOptions) -> Result<(), SysexitsError> {
        use SnapshotCommand::*;
        match self {
            Snapshot(urls) => snapshot(urls, &flags).await,
            List => list(&flags).await,
            Log { url } => log(&url, &flags).await,
            Compact { urls } => compact(&urls, &flags).await,
        }
    }
}

mod compact;
pub use compact::*;

mod list;
pub use list::*;

mod log;
pub use log::*;

mod snapshot;
pub use snapshot::*;
