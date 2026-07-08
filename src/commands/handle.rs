// This is free and unencumbered software released into the public domain.

use clientele::{StandardOptions, SysexitsError, crates::clap::Subcommand};

#[derive(Debug, Subcommand)]
pub enum HandleCommand {}

impl HandleCommand {
    pub async fn run(&self, _flags: &StandardOptions) -> Result<(), SysexitsError> {
        Ok(())
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
