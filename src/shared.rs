// This is free and unencumbered software released into the public domain.

use crate::Result;
use asimov_module::{ModuleManifest, resolve::Module};
use clientele::{Subcommand, SubcommandsProvider, SysexitsError::*};
use color_print::{ceprintln, cstr};
use std::rc::Rc;

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
) -> Result<Vec<ModuleManifest>> {
    let modules = registry
        .installed_modules()
        .await
        .map_err(|e| {
            ceprintln!("<s,r>error:</> unable to access installed modules: {e}");
            match e {
                asimov_registry::error::InstalledModulesError::DirIo(_, err)
                    if err.kind() == std::io::ErrorKind::NotFound =>
                {
                    ceprintln!("{NO_MODULES_FOUND_HINT}");
                },
                _ => (),
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
                    .any(|program| program.split('-').next_back().is_some_and(|p| p == filter))
            } else {
                true
            }
        })
        .collect();

    Ok(modules)
}

pub async fn pick_module(
    registry: &asimov_registry::Registry,
    url: impl AsRef<str>,
    modules: &[Rc<Module>],
    filter: Option<&str>,
) -> Result<Rc<Module>> {
    let url = url.as_ref();

    if let Some(filter) = filter {
        let module = modules.iter().find(|m| m.name == filter).ok_or_else(|| {
                ceprintln!("<s,r>error:</> failed to find a module named `{filter}` that supports handling the URL: `{url}`");
                EX_SOFTWARE
            })?;

        if !registry
            .is_module_enabled(&module.name)
            .await
            .map_err(|e| {
                ceprintln!(
                    "<s,r>error:</> error while checking whether module `{}` is enabled: {e}",
                    module.name
                );
                EX_IOERR
            })?
        {
            ceprintln!(
                "<s,r>error:</> module <s>{}</> is not enabled.",
                module.name
            );
            ceprintln!(
                "<s,dim>hint:</> It can be enabled with: <s>asimov module enable {}</>",
                module.name
            );
            Err(EX_UNAVAILABLE)
        } else {
            Ok(module.clone())
        }
    } else {
        let mut iter = modules.iter();
        loop {
            let module = iter.next().ok_or_else(|| {
                    ceprintln!(
                        "<s,r>error:</> failed to find a module to handle the URL: `{url}`"
                    );
                    let module_count = modules.len();
                    if module_count > 0 {
                        if module_count == 1 {
                            ceprintln!("<s,dim>hint:</> Found <s>{module_count}</> installed module that could handle this URL but is disabled.");
                        } else {
                            ceprintln!("<s,dim>hint:</> Found <s>{module_count}</> installed modules that could handle this URL but are disabled.");
                        }
                        ceprintln!("<s,dim>hint:</> A module can be enabled with: <s>asimov module enable <<module>></>");
                        ceprintln!("<s,dim>hint:</> Available modules:");
                        for module in modules {
                            ceprintln!("<s,dim>hint:</>\t<s>{}</>", module.name);
                        }
                    }
                    EX_UNAVAILABLE
                })?;

            if registry
                .is_module_enabled(&module.name)
                .await
                .map_err(|e| {
                    ceprintln!(
                        "<s,r>error:</> error while checking whether module `{}` is enabled: {e}",
                        module.name
                    );
                    EX_IOERR
                })?
            {
                return Ok(module.clone());
            }
        }
    }
}
