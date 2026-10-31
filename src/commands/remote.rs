// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::string::String;
use asimov_directory::fs::StateDirectory;
use asimov_remote::RemotesRegistry;
use clientele::{StandardOptions, SysexitsError::*, crates::clap::Subcommand};
use std::io::{self, Write};

/// Commands for managing tracked remotes.
#[derive(Debug, Subcommand)]
pub enum RemoteCommand {
    /// List the set of tracked remotes
    ///
    /// Prints names in sorted order. With --verbose, includes each remote's URL.
    #[clap(alias = "ls")]
    List {},

    /// Add a remote named NAME for URL
    Add {
        /// The name of the remote to add
        name: String,

        /// The HTTP(S) URL of the remote
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

    /// Check the remote named NAME with a signed OPTIONS request
    ///
    /// Verifies the signed response. With --verbose, reports success and RTT
    /// on standard error; otherwise success is silent.
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

fn open() -> Result<RemotesRegistry, BoxError> {
    let state = StateDirectory::from_env().map_err(|e| report("open state directory", e))?;
    RemotesRegistry::open(&state).map_err(|e| report("open remote storage", e))
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
