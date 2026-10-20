// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::string::String;
use clientele::{StandardOptions, crates::clap::Subcommand};

/// Commands for managing tracked remotes.
#[derive(Debug, Subcommand)]
pub enum RemoteCommand {
    /// List the set of tracked remotes
    #[clap(alias = "ls")]
    List {},

    /// Add a remote named NAME for URL
    Add {
        /// The name of the remote to add
        name: String,

        /// The URL of the remote
        url: String,
    },

    /// Rename the remote named OLD to NEW
    #[clap(alias = "mv")]
    Rename {
        /// The current name of the remote
        old: String,

        /// The new name of the remote
        new: String,
    },

    /// Remove the remote named NAME
    #[clap(alias = "rm")]
    Remove {
        /// The name of the remote to remove
        name: String,
    },

    /// Ping the remote named NAME
    #[clap(alias = "ping")]
    Check {
        /// The name of the remote to ping
        name: String,
    },
}

impl RemoteCommand {
    /// Dispatches a remote command to its handler.
    pub async fn run(self, flags: &StandardOptions) -> Result<(), BoxError> {
        use RemoteCommand::*;
        match self {
            List {} => list(flags).await,
            Add { name, url } => add(name, url, flags).await,
            Rename { old, new } => rename(old, new, flags).await,
            Remove { name } => remove(name, flags).await,
            Check { name } => check(name, flags).await,
        }
    }
}

mod add;
pub use add::*;

mod check;
pub use check::*;

mod list;
pub use list::*;

mod remove;
pub use remove::*;

mod rename;
pub use rename::*;
