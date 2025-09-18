// This is free and unencumbered software released into the public domain.

use crate::Result;
use clientele::{Subcommand, SubcommandsProvider, SysexitsError::*};

/// Locates the given subcommand or prints an error.
pub fn locate_subcommand(name: &str) -> Result<Subcommand> {
    match SubcommandsProvider::find("asimov-", name) {
        Some(cmd) => Ok(cmd),
        None => {
            eprintln!("{}: command not found: {}{}", "asimov", "asimov-", name);
            Err(EX_UNAVAILABLE)
        },
    }
}
