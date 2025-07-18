// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
    commands::External,
    shared::{build_resolver, normalize_url},
};
use color_print::ceprintln;
use miette::Result;

pub fn list(
    urls: &Vec<String>,
    module: Option<&str>,
    limit: Option<usize>,
    output: Option<&str>,
    flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let resolver = build_resolver("cataloger").map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build a resolver: {e}");
        EX_UNAVAILABLE
    })?;

    for url in urls {
        if flags.verbose > 1 {
            ceprintln!("<s,c>»</> Cataloging `{}`...", url);
        }

        let url = normalize_url(url);

        let modules = resolver.resolve(&url).unwrap(); // FIXME

        let module = if let Some(want) = module {
            modules.iter().find(|m| m.name == want).ok_or_else(|| {
                ceprintln!("<s,r>error:</> failed to find a module named `{want}` that supports cataloging the URL: `{url}`");
                EX_SOFTWARE
            })?
        } else {
            modules.first().ok_or_else(|| {
                ceprintln!("<s,r>error:</> failed to find a module to catalog the URL: `{url}`");
                EX_SOFTWARE
            })?
        };

        let subcommand = format!("{}-cataloger", module.name);

        let cmd = External {
            is_debug: flags.debug,
            pipe_output: false,
        };

        let mut args = vec![];
        if let Some(limit) = limit {
            args.push(format!("--limit={}", limit));
        }
        if let Some(output) = output {
            args.push(format!("--output={}", output));
        }
        args.push(url.to_owned());

        let code = cmd.execute(&subcommand, args).map(|result| result.code)?;
        if code.is_failure() {
            return Err(code);
        }

        if flags.verbose > 0 {
            ceprintln!("<s,g>✓</> Cataloged `{}`.", url);
        }
    }

    Ok(())
}
