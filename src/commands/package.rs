// This is free and unencumbered software released into the public domain.

use clientele::{StandardOptions, crates::clap::Subcommand};
use core::error::Error;
use std::string::String;

#[derive(Debug, Subcommand)]
pub enum PackageCommand {
    /// Initialize `.asimov/module.yaml`.
    Init {
        /// The name of the module
        name: Option<String>,
    },

    /// Check the package directory for missing files/directories.
    Check {},

    /// Display the package directory tree.
    Tree {},
}

impl PackageCommand {
    pub async fn run(&self, flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
        use PackageCommand::*;
        match self {
            Init { name } => init(name, flags).await,
            Check {} => check(flags).await,
            Tree {} => tree(flags).await,
        }
    }
}

mod check;
pub use check::*;

mod init;
pub use init::*;

mod tree;
pub use tree::*;
