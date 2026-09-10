// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, SysexitsError::*, shared};
use asimov_module::{ModuleName, normalization::normalize_url, resolve::Resolver};
use asimov_runner::{FetcherOptions, GraphOutput};
use clientele::crates::clap::Args;
use color_print::ceprintln;
use miette::Result;
use std::io::Write;

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

    /// Filter JSON output using a jq expression.
    #[arg(long, value_name = "EXPR")]
    jq: Option<String>,

    #[clap(flatten)]
    cache: super::cache::CacheArgs,
    urls: Vec<String>,
}

pub async fn fetch(args: SourceFetchArgs, flags: &StandardOptions) -> Result<(), BoxError> {
    let jq = shared::compile_jq(args.jq.as_deref())?;
    let registry = asimov_registry::Registry::default();

    let installed_modules = shared::installed_modules(&registry, Some("fetcher")).await?;

    let resolver = Resolver::try_from_iter(installed_modules.iter()).map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build resolver: {e}");
        EX_UNAVAILABLE
    })?;

    let mut fetchers = Vec::with_capacity(args.urls.len());
    for input_url in args.urls {
        let input_url = normalize_url(&input_url).unwrap_or_else(|e| {
            if flags.verbose > 1 {
                ceprintln!(
                    "<s,y>warning:</> using unmodified URL <s>{input_url}</>; normalization failed: {e}"
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

        let fetcher = asimov_runner::Fetcher::new(
            format!("asimov-{}-fetcher", module.name),
            &input_url,
            GraphOutput::Captured,
            FetcherOptions::builder()
                .maybe_output(args.output.as_deref())
                .maybe_other(args.cache.max_age_option())
                .maybe_other(args.cache.deadline_option())
                .maybe_other(flags.debug.then_some("--debug"))
                .build(),
        );

        fetchers.push((input_url, fetcher));
    }

    let verbose = flags.verbose;

    let tasks: Vec<_> = fetchers
        .into_iter()
        .map(|(url, mut fetcher)| (url, tokio::spawn(async move { fetcher.execute().await })))
        .collect();

    let mut failed = false;
    for (url, task) in tasks {
        if verbose > 1 {
            ceprintln!("<s,c>»</> Fetching <s>{}</>...", url);
        }
        match task.await? {
            Ok(output) => {
                let mut stdout = std::io::stdout().lock();
                if let Some(filter) = jq.as_ref() {
                    for value in shared::filter_json(filter, output.into_inner()).map_err(|e| {
                        ceprintln!("<s,r>error:</> jq filtering failed for <s>{url}</>: {e}");
                        EX_DATAERR
                    })? {
                        writeln!(stdout, "{value}")?;
                    }
                } else {
                    stdout.write_all(&output.into_inner())?;
                }
                stdout.flush()?;
                if verbose > 0 {
                    ceprintln!("<s,g>✓</> Fetched <s>{}</>.", url);
                }
            },
            Err(err) => {
                failed = true;
                ceprintln!("<s,r>error:</> fetcher execution failed for <s>{url}</>: {err}");
            },
        }
    }

    if failed {
        Err(EX_UNAVAILABLE.into())
    } else {
        Ok(())
    }
}
