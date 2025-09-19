// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
    shared,
};
use asimov_module::{resolve::Resolver, url::normalize_url};
use asimov_runner::{FetcherOptions, GraphOutput};
use color_print::ceprintln;
use miette::Result;

pub async fn fetch(
    input_urls: &[String],
    module: Option<&str>,
    output: Option<&str>,
    flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let registry = asimov_registry::Registry::default();

    let installed_modules = shared::installed_modules(&registry, Some("fetcher")).await?;

    let resolver = Resolver::try_from_iter(installed_modules.iter()).map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build resolver: {e}");
        EX_UNAVAILABLE
    })?;

    for input_url in input_urls {
        if flags.verbose > 1 {
            ceprintln!("<s,c>»</> Fetching `{}`...", input_url);
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

        let mut fetcher = asimov_runner::Fetcher::new(
            format!("asimov-{}-fetcher", module.name),
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
