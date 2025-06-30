// This is free and unencumbered software released into the public domain.

use crate::Result;
use asimov_env::paths::asimov_root;
use asimov_module::{models::ModuleManifest, resolve::Resolver};
use clientele::{Subcommand, SubcommandsProvider, SysexitsError::*};
use miette::{IntoDiagnostic, miette};

pub(crate) fn build_resolver(pattern: &str) -> miette::Result<Resolver> {
    let mut resolver = Resolver::new();

    let module_dir_path = asimov_root().join("modules");
    let module_dir = std::fs::read_dir(&module_dir_path)
        .map_err(|e| miette!("Failed to read module manifest directory: {e}"))?
        .filter_map(Result::ok);

    for entry in module_dir {
        let filename = entry.file_name();
        let filename = filename.to_string_lossy();
        if !filename.ends_with(".yml") && !filename.ends_with(".yaml") {
            continue;
        }
        let file = std::fs::File::open(entry.path()).into_diagnostic()?;

        let manifest: ModuleManifest = serde_yml::from_reader(file).map_err(|e| {
            miette!(
                "Invalid module manifest at `{}`: {}",
                entry.path().display(),
                e
            )
        })?;

        if !manifest
            .provides
            .programs
            .iter()
            .any(|program| program.split('-').next_back().is_some_and(|p| p == pattern))
        {
            continue;
        }

        resolver
            .insert_manifest(&manifest)
            .map_err(|e| miette!("{e}"))?;
    }

    Ok(resolver)
}

/// Locates the given subcommand or prints an error.
pub fn locate_subcommand(name: &str) -> Result<Subcommand> {
    let libexec = asimov_root().join("libexec");
    if libexec.exists() {
        let file = std::fs::read_dir(libexec)?
            .filter_map(Result::ok)
            .find(|entry| {
                entry.file_name().to_str().is_some_and(|filename| {
                    filename.starts_with("asimov-") && filename.ends_with(name)
                })
            });
        if let Some(entry) = file {
            return Ok(Subcommand {
                name: format!("asimov-{}", name),
                path: entry.path(),
            });
        }
    }

    match SubcommandsProvider::find("asimov-", name) {
        Some(cmd) => Ok(cmd),
        None => {
            eprintln!("{}: command not found: {}{}", "asimov", "asimov-", name);
            Err(EX_UNAVAILABLE)
        }
    }
}
