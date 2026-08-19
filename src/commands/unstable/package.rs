// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use asimov_module::ModuleName;
use clientele::{StandardOptions, crates::clap::Subcommand};
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

    /// Scaffold a new module
    #[cfg(feature = "package-scaffold")]
    Scaffold {
        /// The module's short name, e.g. `widget` for `asimov-widget-module`
        name: ModuleName,

        /// The target directory to create the module in.
        /// Defaults to `./asimov-<name>-module`.
        #[arg(long)]
        dir: Option<String>,

        /// Scaffold a program of this kind, e.g. `fetcher` for
        /// `asimov-widget-fetcher`. May be repeated to scaffold several
        /// programs at once.
        #[arg(long, default_value = "emitter")]
        program: Vec<String>,

        /// A short summary of what this module does.
        /// Defaults to a summary derived from the module's name.
        #[arg(long)]
        summary: Option<String>,
    },
}

impl PackageCommand {
    pub async fn run(&self, flags: &StandardOptions) -> Result<(), BoxError> {
        use PackageCommand::*;
        match self {
            Init { name } => init(name, flags).await,
            Check {} => check(flags).await,
            Tree {} => tree(flags).await,
            #[cfg(feature = "package-scaffold")]
            Scaffold {
                name,
                dir,
                program,
                summary,
            } => new(name, dir.as_deref(), program, summary.as_deref(), flags).await,
        }
    }
}

mod check;
pub use check::*;

mod init;
pub use init::*;

mod tree;
pub use tree::*;

#[cfg(feature = "package-scaffold")]
mod scaffold;
#[cfg(feature = "package-scaffold")]
pub use scaffold::*;
