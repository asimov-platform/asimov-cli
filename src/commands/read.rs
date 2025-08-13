// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
    commands::External,
    shared::{build_resolver, locate_subcommand, normalize_url},
};
use asimov_module::{ModuleManifest, resolve::Resolver};
use asimov_runner::{AnyInput, GraphOutput, ReaderOptions};
use color_print::ceprintln;
use miette::Result;

pub async fn read(
    input_urls: &Vec<String>,
    module: Option<&str>,
    flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let installer = asimov_installer::Installer::default();
    let installed_modules: Vec<ModuleManifest> = installer
        .installed_modules()
        .await
        .map_err(|e| {
            ceprintln!("<s,r>error:</> unable to access installed modules: {e}");
            match e {
                asimov_registry::error::InstalledModulesError::DirIo(_, err)
                    if err.kind() == std::io::ErrorKind::NotFound => {
                        ceprintln!("<s,dim>hint:</> There appears to be no installed modules.");
                        ceprintln!("<s,dim>hint:</> Modules may be discovered either on the site <u>https://asimov.directory/modules</>");
                        ceprintln!("<s,dim>hint:</> or on the GitHub organization <u>https://github.com/asimov-modules</>");
                        ceprintln!("<s,dim>hint:</> and installed with `asimov module install <<module>>`");
                    },
                _ => (),
            };
            EX_UNAVAILABLE
        })?
        .into_iter()
        .map(|manifest| manifest.manifest)
        .filter(|manifest| {
            manifest
                .provides
                .programs
                .iter()
                .any(|program| program.ends_with("-reader"))
        })
        .collect();

    let resolver = Resolver::try_from_iter(installed_modules.iter()).map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build resolver: {e}");
        EX_UNAVAILABLE
    })?;

    for input_url in input_urls {
        if flags.verbose > 1 {
            ceprintln!("<s,c>»</> Reading `{}`...", input_url);
        }

        let input_url = normalize_url(input_url);

        let modules = resolver.resolve(&input_url).unwrap(); // FIXME

        let module = if let Some(want) = module {
            let module = modules.iter().find(|m| m.name == want).ok_or_else(|| {
                ceprintln!("<s,r>error:</> failed to find a module named `{want}` that supports reading the URL: `{input_url}`");
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
                        "<s,r>error:</> failed to find a module to read the URL: `{input_url}`"
                    );
                    let module_count = modules.len();
                    if module_count > 0 {
                        if module_count == 1 {
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

        let mut reader = asimov_runner::Reader::new(
            format!("asimov-{}-reader", module.name),
            AnyInput::Ignored, // FIXME: &input_url,
            GraphOutput::Inherited,
            ReaderOptions::builder()
                // TODO: .maybe_input(input)
                // TODO: .maybe_output(output)
                .maybe_other(flags.debug.then_some("--debug"))
                .build(),
        );

        let _ = reader.execute().await.expect("should execute reader");

        if flags.verbose > 0 {
            ceprintln!("<s,g>✓</> Read `{}`.", input_url);
        }
    }

    Ok(())
}
