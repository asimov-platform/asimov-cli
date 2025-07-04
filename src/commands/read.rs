// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
    commands::External,
    shared::{build_resolver, normalize_url},
};
use color_print::ceprintln;
use miette::Result;

pub fn read(
    urls: &Vec<String>,
    module: Option<&str>,
    flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let resolver = build_resolver("reader").map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build a resolver: {e}");
        EX_UNAVAILABLE
    })?;

    for url in urls {
        if flags.verbose > 1 {
            ceprintln!("<s,c>»</> Reading `{}`...", url);
        }

        let url = normalize_url(url);

        let modules = resolver.resolve(&url)?;

        let module = if let Some(want) = module {
            modules.iter().find(|m| m.name == want).ok_or_else(|| {
                ceprintln!("<s,r>error:</> failed to find a module named `{want}` that supports reading the URL: `{url}`");
                EX_SOFTWARE
            })?
        } else {
            modules.first().ok_or_else(|| {
                ceprintln!("<s,r>error:</> failed to find a module to read the URL: `{url}`");
                EX_SOFTWARE
            })?
        };

        let subcommand = format!("{}-reader", module.name);

        let cmd = External {
            is_debug: flags.debug,
            pipe_output: false,
        };

        let code = cmd
            .execute(&subcommand, &[url.to_owned()])
            .map(|result| result.code)?;
        if code.is_failure() {
            return Err(code);
        }

        if flags.verbose > 0 {
            ceprintln!("<s,g>✓</> Read `{}`.", url);
        }
    }

    Ok(())
}
