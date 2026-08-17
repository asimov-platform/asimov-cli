// This is free and unencumbered software released into the public domain.

use clap::ValueEnum;
use clientele::{StandardOptions, crates::clap::Subcommand};
use core::error::Error;

#[derive(Debug, Subcommand)]
pub enum ProxyCommand {
    /// Run an OpenAI-compatible endpoint at <http://127.0.0.1:1920>.
    ///
    /// Requests are proxied to enabled providers (currently always OpenRouter).
    ///
    /// Reads OPENROUTER_API_KEY for the OpenRouter API key (required).
    ///
    /// Reads ASIMOV_PROXY_PORT for the port to bind to (default: 1920).
    ///
    /// Reads ASIMOV_PROXY_HOST for the host to bind to (default: 127.0.0.1).
    #[clap(aliases = ["run"])]
    Serve {},

    /// Configure local applications to use the proxy endpoint.
    Config { app: ProxyConfigTarget },

    /// Configure local applications to use the proxy endpoint.
    Install { apps: Vec<ProxyInstallTarget> },
}

impl ProxyCommand {
    pub async fn run(&self, flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
        use ProxyCommand::*;
        match self {
            Serve {} => serve(flags).await,
            Config { app } => config(app, flags).await,
            Install { apps } => install(apps, flags).await,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ProxyConfigTarget {
    /// LangChain (https://langchain.com).
    Langchain,

    /// LlamaIndex (https://llamaindex.ai).
    Llamaindex,

    /// Zed (https://zed.dev).
    Zed,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ProxyInstallTarget {
    /// Cursor (https://zed.dev).
    #[cfg(feature = "unstable")]
    Cursor,

    /// Obsidian (https://obsidian.md).
    #[cfg(feature = "unstable")]
    Obsidian,

    /// Visual Studio Code (https://code.visualstudio.com).
    #[cfg(feature = "unstable")]
    VSCode,

    /// Zed (https://zed.dev).
    Zed,
}

mod config;
pub use config::*;

mod install;
pub use install::*;

mod serve;
pub use serve::*;
