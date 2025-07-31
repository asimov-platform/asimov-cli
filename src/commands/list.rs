// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
    commands::External,
    shared::{build_resolver, locate_subcommand, normalize_url},
};
use asimov_module::{ModuleManifest, resolve::Resolver};
use asimov_runner::{CatalogerOptions, GraphOutput};
use color_print::ceprintln;
use miette::Result;

pub async fn list(
    input_urls: &Vec<String>,
    module: Option<&str>,
    limit: Option<usize>,
    output: Option<&str>,
    flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let installer = asimov_installer::Installer::default();
    let installed_modules: Vec<ModuleManifest> = installer
        .installed_modules()
        .await
        .map_err(|e| {
            ceprintln!("<s,r>error:</> unable to access installed modules: {e}");
            EX_UNAVAILABLE
        })?
        .into_iter()
        .map(|manifest| manifest.manifest)
        .filter(|manifest| {
            manifest
                .provides
                .programs
                .iter()
                .any(|program| program.ends_with("-cataloger"))
        })
        .collect();

    let resolver = Resolver::try_from_iter(installed_modules.iter()).map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build resolver: {e}");
        EX_UNAVAILABLE
    })?;

    for input_url in input_urls {
        if flags.verbose > 1 {
            ceprintln!("<s,c>»</> Cataloging `{}`...", input_url);
        }

        let input_url = normalize_url(input_url);

        let modules = resolver.resolve(&input_url).map_err(|e| {
            ceprintln!("<s,r>error:</> unable to handle URL `{input_url}`: {e}");
            EX_USAGE
        })?;

        let module = if let Some(want) = module {
            let module = modules.iter().find(|m| m.name == want).ok_or_else(|| {
                ceprintln!("<s,r>error:</> failed to find a module named `{want}` that supports cataloging the URL: `{input_url}`");
                EX_SOFTWARE
            })?;

            if installer
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
                module
            } else {
                ceprintln!(
                    "<s,r>error:</> module <s>{}</> is not enabled.",
                    module.name
                );
                ceprintln!(
                    "<s,dim>hint:</> It can be enabled with: `asimov module enable {}`",
                    module.name
                );
                return Err(EX_UNAVAILABLE);
            }
        } else {
            let mut iter = modules.iter();
            loop {
                let module = iter.next().ok_or_else(|| {
                    ceprintln!(
                        "<s,r>error:</> failed to find a module to catalogue the URL: `{input_url}`"
                    );
                    let module_count = modules.len();
                    if module_count > 0 {
                        if modules.len() == 1 {
                            ceprintln!("<s,dim>hint:</> Found <s>{module_count}</> installed module that could handle this URL but is disabled.");
                        } else {
                            ceprintln!("<s,dim>hint:</> Found <s>{module_count}</> installed modules that could handle this URL but are disabled.");
                        }
                        ceprintln!("<s,dim>hint:</> A module can be enabled with: `asimov module enable <<module>>`");
                        ceprintln!("<s,dim>hint:</> Available modules:");
                        for module in &modules {
                            ceprintln!("<s,dim>hint:</>\t<s>{}</>", module.name);
                        }
                    }
                    EX_UNAVAILABLE
                })?;

                if installer
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
                    break module;
                }
            }
        };

        let mut cataloger = asimov_runner::Cataloger::new(
            format!("asimov-{}-cataloger", module.name),
            &input_url,
            GraphOutput::Inherited,
            CatalogerOptions::builder()
                .maybe_limit(limit)
                .maybe_output(output)
                .maybe_other(flags.debug.then_some("--debug"))
                .build(),
        );

        let _ = cataloger.execute().await.expect("should execute cataloger");

        if flags.verbose > 0 {
            ceprintln!("<s,g>✓</> Cataloged `{}`.", input_url);
        }
    }

    Ok(())
}
