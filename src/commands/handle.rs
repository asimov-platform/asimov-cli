// This is free and unencumbered software released into the public domain.

use asimov_id::{Handle, Id, PublicKey, PublicKeyEncoding};
use clientele::{StandardOptions, SysexitsError, crates::clap::Subcommand};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum HandleCommand {
    Add {
        /// The handle
        handle: Handle,

        /// The public keys of the endpoints to associate with the handle
        endpoints: Vec<PublicKey>,
    },

    Export {},

    Import {
        /// The paths to the input files (default: /dev/stdin)
        inputs: Vec<PathBuf>,
    },

    #[clap(aliases = ["ls"])]
    List {},

    #[clap(aliases = ["rm", "delete", "del"])]
    Remove {
        /// The handle
        handle: Handle,

        /// The public keys of the endpoints to disassociate with the handle
        endpoints: Vec<PublicKey>,
    },

    /// Resolve a handle into a set of peer IDs
    #[clap(aliases = ["lookup"])]
    Resolve {
        /// The handle to resolve
        handle: Id,

        /// The output format for the public key (default: asimov)
        #[clap(short, long)]
        format: Option<PublicKeyEncoding>,
    },
}

impl HandleCommand {
    pub async fn run(&self, flags: &StandardOptions) -> Result<(), SysexitsError> {
        match self {
            HandleCommand::Add { handle, endpoints } => add(handle, endpoints, flags).await,
            HandleCommand::Export {} => export(flags).await,
            HandleCommand::Import { inputs } => import(inputs, flags).await,
            HandleCommand::List {} => list(flags).await,
            HandleCommand::Remove { handle, endpoints } => remove(handle, endpoints, flags).await,
            HandleCommand::Resolve { handle, format } => resolve(handle, format, flags).await,
        }
    }
}

mod add;
pub use add::*;

mod export;
pub use export::*;

mod import;
pub use import::*;

mod list;
pub use list::*;

mod remove;
pub use remove::*;

mod resolve;
pub use resolve::*;
