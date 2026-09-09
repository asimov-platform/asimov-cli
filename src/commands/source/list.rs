// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, SysexitsError::*, shared};
use asimov_module::{ModuleName, normalization::normalize_url, resolve::Resolver};
use asimov_runner::{CatalogerOptions, GraphOutput};
use clientele::sort::SortKeys;
use color_print::ceprintln;
use miette::Result;

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
    let installed_modules = shared::installed_modules(&registry, Some("cataloger")).await?;

    let resolver = Resolver::try_from_iter(installed_modules.iter()).map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build resolver: {e}");
        EX_UNAVAILABLE
    })?;

    let mut catalogers = Vec::with_capacity(input_urls.len());
    for input_url in input_urls {
        if flags.verbose > 1 {
            ceprintln!("<s,c>»</> Cataloging <s>{}</>...", input_url);
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

        let module =
            shared::pick_module(&registry, &input_url, modules.as_slice(), module.as_deref())
                .await?;

        let cataloger = asimov_runner::Cataloger::new(
            format!("asimov-{}-cataloger", module.name),
            &input_url,
            if jq.is_some() {
                GraphOutput::Captured
            } else {
                GraphOutput::Inherited
            },
            CatalogerOptions::builder()
                .maybe_sort(sort.clone())
                .maybe_offset(offset)
                .maybe_limit(limit)
                .maybe_output(output.as_deref())
                .maybe_other(cache.max_age_option())
                .maybe_other(cache.deadline_option())
                .maybe_other(flags.debug.then_some("--debug"))
                .build(),
        );

        catalogers.push((input_url, cataloger));
    }

    let verbose = flags.verbose;

    let mut js = tokio::task::JoinSet::new();
    for (url, mut cataloger) in catalogers {
        js.spawn(async move {
            cataloger
                .execute()
                .await
                .inspect(|_| {
                    if verbose > 0 {
                        ceprintln!("<s,g>✓</> Cataloged <s>{}</>.", url)
                    }
                })
                .inspect_err(|err| {
                    ceprintln!("<s,r>error:</> cataloger execution failed for <s>{url}</>: {err}")
                })
        });
    }

    let outputs = js.join_all().await;
    let failed = outputs.iter().any(Result::is_err);

    if let Some(filter) = jq.as_ref() {
        for output in outputs.into_iter().flatten() {
            for value in shared::filter_json(filter, output.into_inner()).map_err(|e| {
                ceprintln!("<s,r>error:</> jq filtering failed: {e}");
                EX_DATAERR
            })? {
                println!("{value}");
            }
        }
    }

    if failed {
        Err(EX_UNAVAILABLE.into())
    } else {
        Ok(())
    }
}
