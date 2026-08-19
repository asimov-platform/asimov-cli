// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions};
use std::fs::exists;
use tracing::{error, warn};

pub async fn check(_flags: &StandardOptions) -> Result<(), BoxError> {
    if !exists(".asimov")? {
        error!("Missing ASIMOV configuration directory `{}/`.", ".asimov");
    }

    if !exists(".asimov/module.yaml")? {
        error!("Missing ASIMOV manifest file `{}`.", ".asimov/module.yaml");
    }

    if !exists(".cargo")? {
        warn!("Missing Cargo configuration directory `{}/`.", ".cargo");
    }

    if !exists(".cargo/config.toml")? {
        warn!(
            "Missing Cargo configuration file `{}`.",
            ".cargo/config.toml"
        );
    }

    for path in [".gitattributes", ".gitignore"] {
        if !exists(path)? {
            warn!("Missing Git configuration file `{}`.", path);
        }
    }

    for path in [".github", ".github/workflows"] {
        if !exists(path)? {
            warn!("Missing GitHub configuration directory `{}/`.", path);
        }
    }

    if !exists(".github/dependabot.yaml")? {
        warn!(
            "Missing Dependabot configuration file `{}`.",
            ".github/dependabot.yaml"
        );
    }

    for path in [
        ".github/workflows/ci.yaml",
        ".github/workflows/release.yaml",
    ] {
        if !exists(path)? {
            warn!("Missing GitHub Actions workflow file `{}`.", path);
        }
    }

    if !exists(".config")? {
        warn!("Missing general configuration directory `{}/`.", ".config");
    }

    if !exists(".config/mise.toml")? {
        warn!("Missing mise configuration file `{}`.", ".config/mise.toml");
    }

    if !exists(".config/readmer/")? {
        warn!(
            "Missing Readmer configuration directory `{}`.",
            ".config/readmer/"
        );
    }

    if !exists(".rustfmt.toml")? {
        warn!("Missing Rustfmt configuration file `{}`.", ".rustfmt.toml");
    }

    if !exists("AUTHORS")? {
        warn!("Missing authors file `{}`.", "AUTHORS");
    }

    if !exists("Cargo.toml")? {
        error!("Missing Cargo manifest file `{}`.", "Cargo.toml");
    }

    if !exists("CHANGES.md")? {
        warn!("Missing changelog file `{}`.", "CHANGES.md");
    }

    if !exists("Makefile")? {
        warn!("Missing Makefile `{}`.", "Makefile");
    }

    if !exists("README.md")? {
        warn!("Missing README file `{}`.", "README.md");
    }

    if !exists("UNLICENSE")? {
        warn!("Missing license file `{}`.", "UNLICENSE");
    }

    if !exists("VERSION")? {
        warn!("Missing version file `{}`.", "VERSION");
    }

    if !exists("etc/")? {
        warn!("Missing et cetera directory `{}`.", "etc/");
    }

    if !exists("src/")? {
        error!("Missing source code directory `{}`.", "src/");
    }

    Ok(())
}
