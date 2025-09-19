// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
    shared,
};
use asimov_module::{resolve::Resolver, url::normalize_url};
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
    let registry = asimov_registry::Registry::default();
    let installed_modules = shared::installed_modules(&registry, Some("cataloger")).await?;

    let resolver = Resolver::try_from_iter(installed_modules.iter()).map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build resolver: {e}");
        EX_UNAVAILABLE
    })?;

    for input_url in input_urls {
        if flags.verbose > 1 {
            ceprintln!("<s,c>»</> Cataloging `{}`...", input_url);
        }

        let input_url = normalize_url(input_url).unwrap_or_else(|e| {
            if flags.verbose > 1 {
                ceprintln!(
                    "<s,y>warning:</> using given unmodified URL, normalization failed: {e}"
                );
            }
            input_url.clone()
        });

        let modules = resolver.resolve(&input_url).map_err(|e| {
            ceprintln!("<s,r>error:</> unable to handle URL `{input_url}`: {e}");
            EX_USAGE
        })?;

        let module = shared::pick_module(&registry, &input_url, modules.as_slice(), module).await?;

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
