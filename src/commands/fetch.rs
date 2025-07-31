// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
    commands::External,
    shared::{locate_subcommand, normalize_url},
};
use asimov_module::{ModuleManifest, resolve::Resolver};
use asimov_runner::{FetcherOptions, GraphOutput};
use color_print::ceprintln;
use miette::Result;

pub async fn fetch(
    input_urls: &Vec<String>,
    module: Option<&str>,
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
                .any(|program| program.ends_with("-fetcher"))
        })
        .collect();

    let resolver = Resolver::try_from_iter(installed_modules.iter()).map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build resolver: {e}");
        EX_UNAVAILABLE
    })?;

    for input_url in input_urls {
        if flags.verbose > 1 {
            ceprintln!("<s,c>»</> Fetching `{}`...", input_url);
        }

        let input_url = normalize_url(input_url);

        let modules = resolver.resolve(&input_url).map_err(|e| {
            ceprintln!("<s,r>error:</> unable to handle URL `{input_url}`: {e}");
            EX_USAGE
        })?;

        let module = if let Some(want) = module {
            let module = modules.iter().find(|m| m.name == want).ok_or_else(|| {
                ceprintln!("<s,r>error:</> failed to find a module named `{want}` that supports fetching the URL: `{input_url}`");
                EX_SOFTWARE
            })?;

            if installer
                .is_module_enabled(&module.name)
                .await
                .map_err(|e| {
                    ceprintln!(
                        "<s,r>error:</> failed to check whether module `{}` is enabled: {e}",
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
                        "<s,r>error:</> failed to find a module to fetch the URL: `{input_url}`"
                    );
                    let module_count = modules.len();
                    if module_count > 0 {
                        if modules.len() == 1 {
                            ceprintln!("<s,dim>hint:</> You have <s>{module_count}</> installed module that could handle this URL but is disabled.");
                        } else {
                            ceprintln!("<s,dim>hint:</> You have <s>{module_count}</> installed modules that could handle this URL but are disabled.");
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
                            "<s,r>error:</> failed to check whether module `{}` is enabled: {e}",
                            module.name
                        );
                        EX_IOERR
                    })?
                {
                    break module;
                }
            }
        };

        // Locate the correct subcommand:
        let subcommand = locate_subcommand(&format!("{}-fetcher", module.name))?;

        let mut fetcher = asimov_runner::Fetcher::new(
            &subcommand.path,
            &input_url,
            GraphOutput::Inherited,
            FetcherOptions::builder()
                .maybe_output(output)
                .maybe_other(flags.debug.then_some("--debug"))
                .build(),
        );

        let _ = fetcher.execute().await.expect("should execute fetcher");

        if flags.verbose > 0 {
            ceprintln!("<s,g>✓</> Fetched `{}`.", input_url);
        }
    }

    Ok(())
}
