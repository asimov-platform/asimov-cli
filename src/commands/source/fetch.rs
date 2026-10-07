// This is free and unencumbered software released into the public domain.

use super::filter::OutputFilter;
use crate::shared::telemetry::{ModuleMetadata, Operation};
use crate::{BoxError, StandardOptions, SysexitsError::*, shared};
use alloc::{format, string::String, vec::Vec};
use asimov_module::{ModuleName, normalization::normalize_url, resolve::Resolver};
use asimov_patterns::{CachingOptions, FilteringOptions, TimingOptions};
use asimov_runner::{ExecutorError, FetcherOptions, GraphOutput, StreamExt};
use clientele::crates::clap::Args;
use color_print::ceprintln;
use miette::Result;

/// See: <https://asimov-specs.github.io/program-patterns/#fetcher-arguments>
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

    #[clap(flatten)]
    filtering: FilteringOptions,

    #[clap(flatten)]
    cache: CachingOptions,

    #[clap(flatten)]
    timing: TimingOptions,

    #[arg(required = true)]
    urls: Vec<String>,
}

/// See: <https://asimov-specs.github.io/program-patterns/#fetcher%E2%91%A0>
///
/// Processes all URLs and preserves the first module failure's sysexits code,
/// including `EX_NOINPUT` for missing resources. Unclassified module failures use
/// `EX_UNAVAILABLE`.
pub async fn fetch(args: SourceFetchArgs, flags: &StandardOptions) -> Result<(), BoxError> {
    let filter = OutputFilter::new(args.filtering)?;
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
                .maybe_other(args.timing.deadline_option())
                .maybe_other(flags.debug.then_some("--debug"))
                .build(),
        );

        fetchers.push((
            input_url,
            ModuleMetadata::new(Operation::Fetch, &module.name),
            fetcher,
        ));
    }

    let verbose = flags.verbose;

    let tasks: Vec<_> = fetchers
        .into_iter()
        .map(|(url, metadata, mut fetcher)| {
            (
                url,
                metadata.start(),
                tokio::spawn(async move { fetcher.execute().await }),
            )
        })
        .collect();

    let mut failure = None;
    for (url, telemetry, task) in tasks {
        if verbose > 1 {
            ceprintln!("<s,c>»</> Fetching <s>{}</>...", url);
        }
        let mut succeeded = false;
        let result: Result<(), BoxError> = async {
            match task.await? {
                Ok(mut output) => {
                    succeeded = true;
                    while let Some(batch) = output.next().await {
                        let batch = match batch {
                            Ok(batch) => batch,
                            Err(err) => {
                                succeeded = false;
                                ceprintln!(
                                    "<s,r>error:</> fetcher execution failed for <s>{url}</>: {err}"
                                );
                                failure.get_or_insert(err);
                                break;
                            },
                        };

                        let mut stdout = std::io::stdout().lock();
                        filter.write_batch(&batch, &url, &mut stdout).await?;
                    }
                    if succeeded && verbose > 0 {
                        ceprintln!("<s,g>✓</> Fetched <s>{}</>.", url);
                    }
                },
                Err(err) => {
                    ceprintln!("<s,r>error:</> fetcher execution failed for <s>{url}</>: {err}");
                    failure.get_or_insert(err);
                },
            }
            Ok(())
        }
        .await;

        telemetry.finish(result.is_ok() && succeeded);

        result?;
    }

    match failure {
        Some(ExecutorError::Failure(code, _)) => Err(code.into()),
        Some(_) => Err(EX_UNAVAILABLE.into()),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clientele::crates::clap::Parser;

    #[derive(Parser)]
    struct Command {
        #[command(flatten)]
        args: SourceFetchArgs,
    }

    #[test]
    fn forwards_only_explicit_options() {
        let args = Command::try_parse_from(["test", "https://example.com"])
            .unwrap()
            .args;
        assert_eq!(args.cache.max_age_option(), None);
        assert_eq!(args.timing.deadline_option(), None);
        let args = Command::try_parse_from([
            "test",
            "--max-age",
            "1h",
            "--deadline",
            "30s",
            "https://example.com",
        ])
        .unwrap()
        .args;
        assert_eq!(args.cache.max_age_option().as_deref(), Some("--max-age=1h"));
        assert_eq!(
            args.timing.deadline_option().as_deref(),
            Some("--deadline=30s")
        );
    }

    #[test]
    fn rejects_invalid_options() {
        for args in [["test", "--max-age", "0s"], ["test", "--deadline", "nope"]] {
            assert!(Command::try_parse_from(args).is_err());
        }
        assert!(Command::try_parse_from(["test", "--wait"]).is_err());
    }

    #[test]
    fn parses_jev_and_jq_together() {
        let args = Command::try_parse_from([
            "test",
            "--jev",
            "The name is Ukrainian",
            "--jq",
            ".name",
            "https://example.com",
        ])
        .unwrap()
        .args;
        assert_eq!(args.filtering.jev.as_deref(), Some("The name is Ukrainian"));
        assert_eq!(args.filtering.jq.as_deref(), Some(".name"));
    }
}
