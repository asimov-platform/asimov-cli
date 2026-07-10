// This is free and unencumbered software released into the public domain.

use asimov_id::{Handle, Id, PublicKey, PublicKeyEncoding};
use clientele::{StandardOptions, crates::clap::Subcommand};
use core::error::Error;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum HandleCommand {
    #[cfg(feature = "unstable")]
    Add {
        /// The handle
        handle: Handle,

        /// The public keys of the endpoints to associate with the handle
        endpoints: Vec<PublicKey>,
    },

    /// Export all handles and their associated endpoints
    Export {},

    #[cfg(feature = "unstable")]
    Import {
        /// The paths to the input files (default: /dev/stdin)
        inputs: Vec<PathBuf>,
    },

    /// List all handles that have endpoints associated with them
    #[clap(aliases = ["ls"])]
    List {},

    #[cfg(feature = "unstable")]
    #[clap(aliases = ["rm", "delete", "del"])]
    Remove {
        /// The handle
        handle: Handle,

        /// The public keys of the endpoints to disassociate with the handle
        endpoints: Vec<PublicKey>,
    },

    /// Resolve a handle into a set of associated endpoints
    #[clap(aliases = ["lookup"])]
    Resolve {
        /// The handle to resolve
        handle: Id,

        /// The encoding format for public keys (default: asimov)
        #[clap(short, long)]
        format: Option<PublicKeyEncoding>,
    },
}

impl HandleCommand {
    pub async fn run(&self, flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
        match self {
            #[cfg(feature = "unstable")]
            HandleCommand::Add { handle, endpoints } => add(handle, endpoints, flags).await,

            HandleCommand::Export {} => export(flags).await,

            #[cfg(feature = "unstable")]
            HandleCommand::Import { inputs } => import(inputs, flags).await,

            HandleCommand::List {} => list(flags).await,

            #[cfg(feature = "unstable")]
            HandleCommand::Remove { handle, endpoints } => remove(handle, endpoints, flags).await,

            HandleCommand::Resolve { handle, format } => resolve(handle, format, flags).await,
        }
    }
}

#[cfg(feature = "unstable")]
mod add;
#[cfg(feature = "unstable")]
pub use add::*;

mod export;
pub use export::*;

#[cfg(feature = "unstable")]
mod import;
#[cfg(feature = "unstable")]
pub use import::*;

mod list;
pub use list::*;

#[cfg(feature = "unstable")]
mod remove;
#[cfg(feature = "unstable")]
pub use remove::*;

mod resolve;
pub use resolve::*;
