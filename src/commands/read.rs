// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
    commands::External,
    shared::{build_resolver, locate_subcommand, normalize_url},
};
use asimov_runner::{AnyInput, GraphOutput, ReaderOptions};
use color_print::ceprintln;
use miette::Result;

pub async fn read(
    input_urls: &Vec<String>,
    module: Option<&str>,
    flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let resolver = build_resolver("reader").map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build a resolver: {e}");
        EX_UNAVAILABLE
    })?;

    for input_url in input_urls {
        if flags.verbose > 1 {
            ceprintln!("<s,c>»</> Reading `{}`...", input_url);
        }

        let input_url = normalize_url(input_url);

        let modules = resolver.resolve(&input_url).unwrap(); // FIXME

        let module = if let Some(want) = module {
            modules.iter().find(|m| m.name == want).ok_or_else(|| {
                ceprintln!("<s,r>error:</> failed to find a module named `{want}` that supports reading the URL: `{input_url}`");
                EX_SOFTWARE
            })?
        } else {
            modules.first().ok_or_else(|| {
                ceprintln!("<s,r>error:</> failed to find a module to read the URL: `{input_url}`");
                EX_SOFTWARE
            })?
        };

        // Locate the correct subcommand:
        let subcommand = locate_subcommand(&format!("{}-reader", module.name))?;

        let mut reader = asimov_runner::Reader::new(
            &subcommand.path,
            AnyInput::Ignored, // FIXME: &input_url,
            GraphOutput::Inherited,
            ReaderOptions::builder()
                // TODO: .maybe_input(input)
                // TODO: .maybe_output(output)
                .maybe_other(flags.debug.then_some("--debug"))
                .build(),
        );

        let _ = reader.execute().await.expect("should execute reader");

        if flags.verbose > 0 {
            ceprintln!("<s,g>✓</> Read `{}`.", input_url);
        }
    }

    Ok(())
}
