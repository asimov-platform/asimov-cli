// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions};
use clientele::crates::clap::Subcommand;
use std::{string::String, vec::Vec};

#[derive(Debug, Subcommand)]
pub enum SnapCommand {
    /// Save a snapshot of a URL, utilizing enabled modules
    #[clap(aliases = ["create", "update"])]
    Save {
        #[clap(flatten)]
        args: SnapSaveArgs,
    },

    /// List all saved snapshots
    List,

    /// Show snapshot log for a given URL
    Log {
        /// URL to show log for
        url: String,
    },

    /// Compact the snapshots for a given URL
    Compact {
        /// URL(s) to compact snapshots for
        urls: Vec<String>,
    },
}

impl SnapCommand {
    pub async fn run(&self, flags: &StandardOptions) -> Result<(), BoxError> {
        use SnapCommand::*;
        match self {
            Save { args } => save(args, flags).await,
            List => list(flags).await,
            Log { url } => log(url, flags).await,
            Compact { urls } => compact(urls, flags).await,
        }
    }
}

mod compact;
pub use compact::*;

mod create;
pub use create::*;

mod list;
pub use list::*;

mod log;
pub use log::*;

mod save;
pub use save::*;
