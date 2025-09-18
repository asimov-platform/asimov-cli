// This is free and unencumbered software released into the public domain.

use crate::Result;
use asimov_module::ModuleManifest;
use clientele::{
    Subcommand, SubcommandsProvider,
    SysexitsError::{self, *},
};
use color_print::{ceprintln, cstr};

/// Locates the given subcommand or prints an error.
pub fn locate_subcommand(name: &str) -> Result<Subcommand> {
    match SubcommandsProvider::find("asimov-", name) {
        Some(cmd) => Ok(cmd),
        None => {
            eprintln!("asimov: command not found: asimov-{}", name);
            Err(EX_UNAVAILABLE)
        },
    }
}

const NO_MODULES_FOUND_HINT: &str = cstr!(
    r#"<s,dim>hint:</> There appears to be no installed modules.
<s,dim>hint:</> Modules may be discovered either on the site <u>https://asimov.directory/modules</>
<s,dim>hint:</> or on the GitHub organization <u>https://github.com/asimov-modules</>
<s,dim>hint:</> and installed with <s>asimov module install <<module>></>"#
);

pub async fn installed_modules(
    registry: &asimov_registry::Registry,
    filter: Option<&str>,
) -> Result<Vec<ModuleManifest>, SysexitsError> {
    let modules = registry
        .installed_modules()
        .await
        .map_err(|e| {
            ceprintln!("<s,r>error:</> unable to access installed modules: {e}");
            if let asimov_registry::error::InstalledModulesError::DirIo(_, err) = e
                && err.kind() == std::io::ErrorKind::NotFound
            {
                ceprintln!("{NO_MODULES_FOUND_HINT}");
            }
            EX_UNAVAILABLE
        })?
        .into_iter()
        .map(|manifest| manifest.manifest)
        .filter(|manifest| {
            if let Some(filter) = filter {
                manifest
                    .provides
                    .programs
                    .iter()
                    .any(|program| program.split("-").last().is_some_and(|p| p == filter))
            } else {
                true
            }
        })
        .collect();

    Ok(modules)
}
