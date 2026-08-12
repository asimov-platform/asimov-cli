// This is free and unencumbered software released into the public domain.

use clientele::{StandardOptions, crates::clap::Subcommand};
use core::error::Error;

#[derive(Debug, Subcommand)]
pub enum ProxyCommand {
    /// Run a local proxy server at http://127.0.0.1:1920
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
