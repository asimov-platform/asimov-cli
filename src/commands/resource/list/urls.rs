// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use crate::shared::{
    execution::{self, Source, SourceResolver},
    filter::OutputFilter,
    telemetry::{ModuleMetadata, Operation},
};
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use asimov_module::{ModuleName, normalization::normalize_url};
use asimov_patterns::{CachingOptions, FilteringOptions, TimingOptions};
use asimov_remote::{ListerRequest, http};
use asimov_runner::{GraphOutput, Lister, ListerOptions, StreamExt};
use clientele::{StandardOptions, sort::SortKeys};
use color_print::ceprintln;
use miette::Result;

#[derive(Clone, Debug, Default, clap::Args)]
pub struct ResourceListArgs {
    /// The collection URL(s) to examine; omit to list tracked resources.
    pub urls: Vec<String>,

    #[clap(flatten)]
    pub cache: CachingOptions,

    #[clap(flatten)]
    pub timing: TimingOptions,

    /// The specific module to use.
    #[clap(long, short = 'M')]
    pub module: Option<ModuleName>,

    /// Sort resources by the specified keys. (Prefix a key with `-` for descending order.)
    #[clap(long, aliases = ["sort-by", "order", "order-by"], value_name = "[+|-]KEY,...", allow_hyphen_values = true)]
    pub sort: Option<SortKeys>,

    /// The index offset of the first output (default: 0).
    #[clap(value_name = "INDEX", long, conflicts_with_all = ["before", "after"])]
    pub offset: Option<usize>,

    /// Select entries before this entry URI, exclusively in the selected order.
    #[arg(long, value_name = "URI", value_parser = parse_cursor)]
    pub before: Option<String>,

    /// Select entries after this entry URI, exclusively in the selected order.
    #[arg(long, value_name = "URI", value_parser = parse_cursor)]
    pub after: Option<String>,

    /// The maximum count of outputs [default: none].
    #[arg(value_name = "COUNT", short = 'n', long)]
    pub limit: Option<usize>,

    /// The output format.
    #[arg(value_name = "FORMAT", short = 'o', long)]
    pub output: Option<String>, // TODO: OutputFormat, default_value = "jsonl"

    #[clap(flatten)]
    pub filtering: FilteringOptions,
}

// Preserve the spelling of entry IDs: validation must not normalize cursors.
fn parse_cursor(value: &str) -> core::result::Result<String, &'static str> {
    if value.is_empty()
        || value.chars().any(|c| c.is_whitespace() || c.is_control())
        || url::Url::parse(value).is_err()
    {
        return Err("expected an absolute entry URI (JSON-LD @id)");
    }
    Ok(value.to_string())
}

/// See: <https://asimov-specs.github.io/program-patterns/#lister>
///
/// Processes all URLs and preserves the first module failure's sysexits code,
/// including `EX_NOINPUT` for missing resources. Unclassified module failures use
/// `EX_UNAVAILABLE`.
pub(super) async fn list(args: ResourceListArgs, flags: &StandardOptions) -> Result<(), BoxError> {
    let ResourceListArgs {
        urls: input_urls,
        module,
        sort,
        offset,
        before,
        after,
        limit,
        output,
        filtering,
        cache,
        timing,
    } = args;
    OutputFilter::validate_json(&filtering, output.as_deref())?;
    let mut filter = None;
    let mut resolver = SourceResolver::new("lister").await?;

    let mut listers = Vec::with_capacity(input_urls.len());
    for input_url in input_urls {
        let input_url = normalize_url(&input_url).unwrap_or_else(|e| {
            if flags.verbose > 1 {
                ceprintln!(
                    "<s,y>warning:</> using unmodified URL <s>{input_url}</>; normalization failed: {e}"
                );
            }
            input_url.clone()
        });

        let source = resolver.resolve(&input_url, module.as_deref()).await?;
        let mut options = ListerOptions::builder()
            .maybe_sort(sort.clone())
            .maybe_offset(offset)
            .maybe_before(before.as_deref())
            .maybe_after(after.as_deref())
            .maybe_limit(limit)
            .maybe_output(output.as_deref())
            .build();
        let (metadata, remote, lister) = match source {
            Source::Local(module) => {
                if filter.is_none() {
                    filter = Some(OutputFilter::new(filtering.clone())?);
                }
                options.other.extend(cache.max_age_option());
                options.other.extend(timing.deadline_option());
                if flags.debug {
                    options.other.push("--debug".into());
                }
                let lister = Lister::new(
                    format!("asimov-{}-lister", module.name),
                    &input_url,
                    GraphOutput::Captured,
                    options,
                );
                (
                    ModuleMetadata::new(Operation::List, &module.name),
                    false,
                    execution::execute(lister, &input_url),
                )
            },
            Source::Relay(executor) => {
                let request = ListerRequest::new(&input_url, options)
                    .with_caching(cache.clone())
                    .with_filtering(filtering.clone())
                    .with_timing(timing.clone());
                (
                    ModuleMetadata::new(Operation::List, "relay"),
                    true,
                    execution::execute(http::Lister::new(*executor, request), &input_url),
                )
            },
        };

        listers.push((input_url, metadata, remote, lister));
    }

    let verbose = flags.verbose;

    let tasks: Vec<_> = listers
        .into_iter()
        .map(|(url, metadata, remote, lister)| {
            (url, metadata.start(), remote, tokio::spawn(lister))
        })
        .collect();

    let mut failure = None;
    for (url, telemetry, remote, task) in tasks {
        if verbose > 1 {
            ceprintln!("<s,c>»</> Listing <s>{}</>...", url);
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
                                    "<s,r>error:</> lister execution failed for <s>{url}</>: {err}"
                                );
                                failure.get_or_insert(err);
                                break;
                            },
                        };

                        let mut stdout = std::io::stdout().lock();
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
                        ceprintln!("<s,g>✓</> Listed <s>{}</>.", url);
                    }
                },
                Err(err) => {
                    ceprintln!("<s,r>error:</> lister execution failed for <s>{url}</>: {err}");
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
        Some(error) => Err(error.exit_code().into()),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Parser, error::ErrorKind};

    #[derive(Debug, Parser)]
    struct Command {
        #[command(flatten)]
        args: ResourceListArgs,
    }

    #[test]
    fn cursors_preserve_uri_spelling_without_an_implicit_offset() {
        let args = Command::try_parse_from([
            "list",
            "--sort=-name",
            "--before=HTTPS://Example.COM/a%2fb?x=a=b#end",
            "--after=urn:entry:1",
            "--limit=5",
            "https://example.com/",
        ])
        .unwrap()
        .args;
        assert!(args.offset.is_none());
        assert_eq!(
            args.before.as_deref(),
            Some("HTTPS://Example.COM/a%2fb?x=a=b#end")
        );
        assert_eq!(args.after.as_deref(), Some("urn:entry:1"));
        assert_eq!(args.limit, Some(5));
        assert!(
            Command::try_parse_from(["list", "https://example.com/"])
                .unwrap()
                .args
                .offset
                .is_none()
        );
    }

    #[test]
    fn rejects_mixed_pagination_and_malformed_cursors_during_parsing() {
        for option in ["--before", "--after"] {
            for offset in ["0", "2"] {
                let error = Command::try_parse_from([
                    "list",
                    option,
                    "urn:entry:1",
                    "--offset",
                    offset,
                    "https://example.com/",
                ])
                .unwrap_err();
                assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
            }
            for uri in [
                "",
                "relative/path",
                "urn:entry:1\n",
                " urn:entry:1",
                "not a URI",
            ] {
                let error = Command::try_parse_from(["list", option, uri, "https://example.com/"])
                    .unwrap_err();
                assert_eq!(error.kind(), ErrorKind::ValueValidation);
            }
        }
    }
}
