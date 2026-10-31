// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use crate::shared::{
    execution::{self, Source, SourceResolver},
    filter::OutputFilter,
    telemetry::{ModuleMetadata, Operation},
};
use alloc::{collections::VecDeque, format, string::String, vec::Vec};
use asimov_module::{ModuleName, normalization::normalize_url};
use asimov_patterns::{CachingOptions, FilteringOptions, TimingOptions};
use asimov_remote::{FetcherRequest, http};
use asimov_runner::{FetcherOptions, GraphOutput, StreamExt};
use clientele::{StandardOptions, crates::clap::Args};
use color_print::ceprintln;
use miette::Result;

/// See: <https://asimov-specs.github.io/program-patterns/#fetcher-arguments>
#[derive(Args, Clone, Debug, Default)]
pub struct ResourceFetchArgs {
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
pub async fn fetch(args: ResourceFetchArgs, flags: &StandardOptions) -> Result<(), BoxError> {
    OutputFilter::validate_json(&args.filtering, args.output.as_deref())?;
    let mut filter = None;
    let mut resolver = SourceResolver::new("fetcher").await?;

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

        let source = resolver.resolve(&input_url, args.module.as_deref()).await?;
        let (metadata, remote, fetcher) = match source {
            Source::Local(module) => {
                if filter.is_none() {
                    filter = Some(OutputFilter::new(args.filtering.clone())?);
                }
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
                (
                    ModuleMetadata::new(Operation::Fetch, &module.name),
                    false,
                    execution::execute(fetcher, &input_url),
                )
            },
            Source::Relay(executor) => {
                let request = FetcherRequest::new(
                    &input_url,
                    FetcherOptions::builder()
                        .maybe_output(args.output.as_deref())
                        .build(),
                )
                .with_caching(args.cache.clone())
                .with_filtering(args.filtering.clone())
                .with_timing(args.timing.clone());
                (
                    ModuleMetadata::new(Operation::Fetch, "relay"),
                    true,
                    execution::execute(http::Fetcher::new(*executor, request), &input_url),
                )
            },
        };

        fetchers.push((input_url, metadata, remote, fetcher));
    }

    let verbose = flags.verbose;

    // Start fetchers in input order, keeping at most one per CPU running,
    // including the one whose output is being written.
    let jobs = std::thread::available_parallelism().map_or(1, core::num::NonZero::get);
    let mut pending = fetchers.into_iter();
    let mut tasks = AbortOnDrop(VecDeque::with_capacity(jobs));
    let mut start_next = |tasks: &mut AbortOnDrop<_, _>| {
        if let Some((url, metadata, remote, fetcher)) = pending.next() {
            tasks
                .0
                .push_back((url, metadata.start(), remote, tokio::spawn(fetcher)));
        }
    };
    for _ in 0..jobs {
        start_next(&mut tasks);
    }

    let mut failure = None;
    while let Some((url, telemetry, remote, task)) = tasks.0.pop_front() {
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

                        let mut stdout = crate::shared::stdout();
                        if remote {
                            execution::write_remote_batch(&batch, &mut stdout)?;
                        } else {
                            filter
                                .as_ref()
                                .expect("local output filter was initialized")
                                .write_batch(&batch, &url, &mut stdout)
                                .await?;
                        }
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
        start_next(&mut tasks);
    }

    match failure {
        Some(error) => Err(error.exit_code().into()),
        None => Ok(()),
    }
}

/// Fetcher tasks still queued for output. Dropping a task's handle would
/// detach it, so on an early return these are aborted instead; dropping
/// their output streams in turn kills the fetcher processes.
struct AbortOnDrop<T, O>(VecDeque<(String, T, bool, tokio::task::JoinHandle<O>)>);

impl<T, O> Drop for AbortOnDrop<T, O> {
    fn drop(&mut self) {
        for (_, _, _, task) in &self.0 {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clientele::crates::clap::Parser;

    #[derive(Parser)]
    struct Command {
        #[command(flatten)]
        args: ResourceFetchArgs,
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
    fn parses_all_filters_together() {
        let args = Command::try_parse_from([
            "test",
            "--jev",
            "The name is Ukrainian",
            "--json",
            "@id,name",
            "--jq",
            ".name",
            "https://example.com",
        ])
        .unwrap()
        .args;
        assert_eq!(args.filtering.jev.as_deref(), Some("The name is Ukrainian"));
        assert_eq!(args.filtering.json.unwrap(), ["@id", "name"]);
        assert_eq!(args.filtering.jq.as_deref(), Some(".name"));
    }
}
