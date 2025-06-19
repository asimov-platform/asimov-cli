// This is free and unencumbered software released into the public domain.

use crate::{commands::External, shared::build_resolver, StandardOptions, SysexitsError};
use color_print::ceprintln;
use miette::Result;

pub fn fetch(urls: &Vec<String>, flags: &StandardOptions) -> Result<(), SysexitsError> {
    let resolver = build_resolver("fetcher").map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build a resolver: {e}");
        SysexitsError::EX_UNAVAILABLE
    })?;

    for url in urls {
        if flags.verbose > 1 {
            ceprintln!("<s><c>»</></> Fetching `{}`...", url);
        }

        let modules = resolver.resolve(url)?;

        match modules.first() {
            Some(module) => {
                let subcommand = format!("{}-fetcher", module.name);

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
            }
            None => {
                ceprintln!(
                    "<s,r>error:</> failed to find a module to fetch the URL: `{}`",
                    url
                );
                return Err(SysexitsError::EX_SOFTWARE);
            }
        }

        if flags.verbose > 0 {
            ceprintln!("<s><g>✓</></> Fetched `{}`.", url);
        }
    }

    Ok(())
}
