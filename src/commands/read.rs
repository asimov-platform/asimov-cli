// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
    shared,
};
use asimov_module::{normalization::normalize_url, resolve::Resolver};
use asimov_runner::{GraphOutput, Input, ReaderOptions};
use color_print::ceprintln;
use miette::Result;

pub async fn read(
    input_urls: &Vec<String>,
    module: Option<&str>,
    flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let registry = asimov_registry::Registry::default();

    let installed_modules = shared::installed_modules(&registry, Some("reader")).await?;

    let resolver = Resolver::try_from_iter(installed_modules.iter()).map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build resolver: {e}");
        EX_UNAVAILABLE
    })?;

    for input_url in input_urls {
        if flags.verbose > 1 {
            ceprintln!("<s,c>»</> Reading <s>{}</> ...", input_url);
        }

        let mime_modules = infer::get_from_path(input_url)
            .inspect_err(|e| {
                if flags.verbose > 1 {
                    ceprintln!(
                        "<s,y>warning:</> failed to determine MIME type of <s>{input_url}</>: {e}"
                    )
                }
            })
            .ok()
            .flatten()
            .map(|t| t.mime_type())
            .and_then(|mt| mt.parse().ok())
            .map(|mime_type| resolver.resolve_content_type(&mime_type))
            .unwrap_or_default();

        let normalized_url = normalize_url(input_url).unwrap_or_else(|e| {
            if flags.verbose > 1 {
                ceprintln!(
                    "<s,y>warning:</> using given unmodified URL, normalization failed: {e}"
                );
            }
            input_url.clone()
        });

        let url_modules = resolver
            .resolve(&normalized_url)
            .inspect_err(|e| {
                if flags.verbose > 1 {
                    ceprintln!(
                        "<s,r>warning:</> failed while resolving URL <s>{normalized_url}</>: {e}"
                    );
                }
            })
            .unwrap_or_default();

        // mime modules first for prioritization
        let modules = [mime_modules, url_modules].concat();

        let module = shared::pick_module(&registry, &input_url, &modules, module).await?;

        let mut reader = asimov_runner::Reader::new(
            format!("asimov-{}-reader", module.name),
            Input::Ignored,
            GraphOutput::Inherited,
            ReaderOptions::builder()
                .other(input_url)
                .maybe_other(flags.debug.then_some("--debug"))
                .build(),
        );

        let mut output = reader.execute().await.expect("should execute reader");

        tokio::io::copy(&mut output, &mut tokio::io::stdout()).await?;

        if flags.verbose > 0 {
            ceprintln!("<s,g>✓</> Read <s>{}</>.", input_url);
        }
    }

    Ok(())
}
