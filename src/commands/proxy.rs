// This is free and unencumbered software released into the public domain.

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
}

impl ProxyCommand {
    pub async fn run(&self, flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
        use ProxyCommand::*;
        match self {
            Serve {} => serve(flags).await,
        }
    }
}

mod serve;
pub use serve::*;
