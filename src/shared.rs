// This is free and unencumbered software released into the public domain.

use std::path::Path;

use crate::Result;
use asimov_env::paths::asimov_root;
use asimov_module::{ModuleManifest, resolve::Resolver};
use clientele::{Subcommand, SubcommandsProvider, SysexitsError::*};
use miette::{IntoDiagnostic, miette};

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
