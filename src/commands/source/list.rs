// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, SysexitsError::*, shared};
use asimov_module::{ModuleName, normalization::normalize_url, resolve::Resolver};
use asimov_runner::{GraphOutput, Lister, ListerOptions};
use clientele::sort::SortKeys;
use color_print::ceprintln;
use miette::Result;
use std::io::Write;

pub async fn list(
    input_urls: Vec<String>,
    module: Option<ModuleName>,
    sort: Option<SortKeys>,
    offset: Option<usize>,
    limit: Option<usize>,
    output: Option<String>,
    jq: Option<String>,
    cache: super::cache::CacheArgs,
    flags: &StandardOptions,
) -> Result<(), BoxError> {
    let jq = shared::compile_jq(jq.as_deref())?;
    let registry = asimov_registry::Registry::default();
    let installed_modules = shared::installed_modules(&registry, Some("lister")).await?;

    let resolver = Resolver::try_from_iter(installed_modules.iter()).map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build resolver: {e}");
        EX_UNAVAILABLE
    })?;

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

        let modules = resolver.resolve(&input_url).map_err(|e| {
            ceprintln!("<s,r>error:</> unable to handle URL <s>{input_url}</>: {e}");
            EX_USAGE
        })?;

        let module =
            shared::pick_module(&registry, &input_url, modules.as_slice(), module.as_deref())
                .await?;

        let lister = Lister::new(
            format!("asimov-{}-lister", module.name),
            &input_url,
            GraphOutput::Captured,
            ListerOptions::builder()
                .maybe_sort(sort.clone())
                .maybe_offset(offset)
                .maybe_limit(limit)
                .maybe_output(output.as_deref())
                .maybe_other(cache.max_age_option())
                .maybe_other(cache.deadline_option())
                .maybe_other(flags.debug.then_some("--debug"))
                .build(),
        );

        listers.push((input_url, lister));
    }

    let verbose = flags.verbose;

    let tasks: Vec<_> = listers
        .into_iter()
        .map(|(url, mut lister)| (url, tokio::spawn(async move { lister.execute().await })))
        .collect();

    let mut failed = false;
    for (url, task) in tasks {
        if verbose > 1 {
            ceprintln!("<s,c>»</> Listing <s>{}</>...", url);
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
                    ceprintln!("<s,g>✓</> Listed <s>{}</>.", url);
                }
            },
            Err(err) => {
                failed = true;
                ceprintln!("<s,r>error:</> lister execution failed for <s>{url}</>: {err}");
            },
        }
    }

    if failed {
        Err(EX_UNAVAILABLE.into())
    } else {
        Ok(())
    }
}
