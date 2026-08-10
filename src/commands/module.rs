// This is free and unencumbered software released into the public domain.

use clientele::{StandardOptions, SysexitsError::*, crates::clap::Subcommand};
use color_print::ceprintln;
use core::error::Error;
use std::{string::String, vec::Vec};

#[derive(Debug, Subcommand)]
pub enum ModuleCommand {
    /// Open the module's package page in a web browser
    #[clap(alias = "open")]
    Browse {
        /// The name of the module to browse
        name: String,
    },

    /// Show or change an installed module's configuration
    #[clap(args_conflicts_with_subcommands = true)]
    Config {
        #[clap(subcommand)]
        command: Option<ConfigCommand>,

        /// The name of the module whose configuration to show
        name: Option<String>,
    },

    /// Disable modules
    Disable {
        /// The names of the modules to disable
        names: Vec<String>,
    },

    /// Print an installed module's documentation
    #[clap(alias = "readme")]
    Doc {
        /// The name of the module whose documentation to print
        name: String,
    },

    /// Enable modules
    Enable {
        /// The names of the modules to enable
        names: Vec<String>,
    },

    /// TBD
    #[cfg(feature = "unstable")]
    #[clap(alias = "which")]
    Find {
        /// The name of the module to find
        name: String,
    },

    /// Inspect a module's manifest, state, and configuration status
    #[clap(alias = "show")]
    Inspect {
        /// The name of the module to inspect
        name: String,

        /// Set the output format [default: cli] [possible values: cli, json]
        #[arg(value_name = "FORMAT", short = 'o', long)]
        output: Option<String>,
    },

    /// Install an available module locally
    Install {
        /// The names of the modules to install
        names: Vec<String>,

        /// Optionally install a specific version instead of latest
        #[arg(long)]
        version: Option<String>,

        /// Optionally specify desired model size to download for module.
        /// Only affects modules which require models.
        #[arg(long)]
        model_size: Option<String>,
    },

    /// Print the module's package link
    #[clap(alias = "url")]
    Link {
        /// The name of the module to link to
        name: String,
    },

    /// List all available and/or installed modules
    #[clap(alias = "ls")]
    List {
        /// Set the output format [default: cli] [possible values: cli, jsonl]
        #[arg(value_name = "FORMAT", short = 'o', long)]
        output: Option<String>,
    },

    /// Scaffold a new module
    #[cfg(feature = "module-new")]
    New {
        /// The module's short name, e.g. `widget` for `asimov-widget-module`
        name: String,

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

    /// Resolve a given URL to modules which can handle it
    Resolve {
        /// The URL to resolve
        url: String,
    },

    /// Search the index of available modules
    Search {
        /// The terms to search for, all of which must match
        #[clap(required = true)]
        query: Vec<String>,

        /// Set the output format [default: cli] [possible values: cli, jsonl]
        #[arg(value_name = "FORMAT", short = 'o', long)]
        output: Option<String>,
    },

    /// Uninstall a currently installed module
    Uninstall {
        /// The names of the modules to uninstall
        names: Vec<String>,
    },

    /// Upgrade currently installed modules
    ///
    /// By default upgrades all installed modules.
    #[clap(alias = "update")]
    Upgrade {
        /// The names of the modules to upgrade
        names: Vec<String>,

        /// Optionally upgrade to a specific version instead of latest
        #[arg(long)]
        version: Option<String>,

        /// Optionally specify desired model size to download for module.
        /// Only affects modules which require models.
        #[arg(long)]
        model_size: Option<String>,
    },
}

impl ModuleCommand {
    pub async fn run(&self, flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
        use ModuleCommand::*;
        match self {
            Browse { name } => browse(name, flags).await,
            Config { command, name } => match (command, name) {
                (Some(command), _) => command.run(flags).await,
                // a bare module name lists its configuration
                (None, Some(name)) => {
                    ConfigCommand::Show {
                        name: name.clone(),
                        output: None,
                    }
                    .run(flags)
                    .await
                },
                (None, None) => {
                    ceprintln!("<s,r>error:</> missing module name or subcommand");
                    ceprintln!(
                        "<s,dim>hint:</> See the available subcommands with: <s>asimov module config --help</>"
                    );
                    Err(EX_USAGE.into())
                },
            },
            Disable { names } => disable(names, flags).await,
            Doc { name } => doc(name, flags).await,
            Enable { names } => enable(names, flags).await,
            #[cfg(feature = "unstable")]
            Find { name } => find(name, flags).await,
            Inspect { name, output } => {
                inspect(name, output.as_deref().unwrap_or("cli"), flags).await
            },
            Install {
                names,
                version,
                model_size,
            } => install(names, version, model_size, flags).await,
            Link { name } => link(name, flags).await,
            List { output } => list(output.as_deref().unwrap_or("cli"), flags).await,
            #[cfg(feature = "module-new")]
            New {
                name,
                dir,
                program,
                summary,
            } => new(name, dir.as_deref(), program, summary.as_deref(), flags).await,
            Resolve { url } => resolve(url, flags).await,
            Search { query, output } => {
                search(query, output.as_deref().unwrap_or("cli"), flags).await
            },
            Uninstall { names } => uninstall(names, flags).await,
            Upgrade {
                names,
                version,
                model_size,
            } => upgrade(names, version, model_size, flags).await,
        }
    }
}

mod browse;
pub use browse::*;

mod config;
pub use config::*;

mod disable;
pub use disable::*;

mod doc;
pub use doc::*;

mod enable;
pub use enable::*;

mod find;
pub use find::*;

mod inspect;
pub use inspect::*;

mod install;
pub use install::*;

mod link;
pub use link::*;

mod list;
pub use list::*;

#[cfg(feature = "module-new")]
mod new;
#[cfg(feature = "module-new")]
pub use new::*;

mod resolve;
pub use resolve::*;

mod search;
pub use search::*;

mod uninstall;
pub use uninstall::*;

mod upgrade;
pub use upgrade::*;
