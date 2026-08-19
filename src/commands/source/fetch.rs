// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, SysexitsError::*, shared};
use asimov_module::{ModuleName, normalization::normalize_url, resolve::Resolver};
use asimov_runner::{FetcherOptions, GraphOutput};
use clientele::crates::clap::Args;
use color_print::ceprintln;
use miette::Result;

#[derive(Args, Clone, Debug, Default)]
pub struct SourceFetchArgs {
    /// Optionally choose the module instead of using module resolution.
    /// The module's manifest must declare support for the URL for the
    /// module to be used.
    #[clap(long, short = 'M')]
    module: Option<ModuleName>,

    /// The output format.
    #[arg(value_name = "FORMAT", short = 'o', long)]
    output: Option<String>,

    urls: Vec<String>,
}

pub async fn fetch(args: SourceFetchArgs, flags: &StandardOptions) -> Result<(), BoxError> {
    let registry = asimov_registry::Registry::default();

    let installed_modules = shared::installed_modules(&registry, Some("fetcher")).await?;

    let resolver = Resolver::try_from_iter(installed_modules.iter()).map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build resolver: {e}");
        EX_UNAVAILABLE
    })?;

    for input_url in args.urls {
        if flags.verbose > 1 {
            ceprintln!("<s,c>»</> Fetching <s>{}</>...", input_url);
        }

        let input_url = normalize_url(&input_url).unwrap_or_else(|e| {
            if flags.verbose > 1 {
                ceprintln!(
                    "<s,y>warning:</> using given unmodified URL, normalization failed: {e}"
                );
            }
            input_url.clone()
        });

        let modules = resolver.resolve(&input_url).map_err(|e| {
            ceprintln!("<s,r>error:</> unable to handle URL <s>{input_url}</>: {e}");
            EX_USAGE
        })?;

        let module = shared::pick_module(
            &registry,
            &input_url,
            modules.as_slice(),
            args.module.as_deref(),
        )
        .await?;

        let mut fetcher = asimov_runner::Fetcher::new(
            format!("asimov-{}-fetcher", module.name),
            &input_url,
            GraphOutput::Inherited,
            FetcherOptions::builder()
                .maybe_output(args.output.as_deref())
                .maybe_other(flags.debug.then_some("--debug"))
                .build(),
        );

        let _ = fetcher.execute().await.map_err(|e| {
            ceprintln!("<s,r>error:</> fetcher execution failed: {e}");
            EX_UNAVAILABLE
        })?;

        if flags.verbose > 0 {
            ceprintln!("<s,g>✓</> Fetched <s>{}</>.", input_url);
        }
    }

    Ok(())
}
