// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError::*};
use color_print::{ceprintln, cprintln};
use core::error::Error;

pub async fn search(
    query: &[String],
    output: &str,
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    let index = asimov_module::Index::fetch().await.map_err(|e| {
        tracing::error!("failed to fetch the module index: {e}");
        EX_UNAVAILABLE
    })?;

    let query = query.join(" ");
    let modules: Vec<_> = index.search(&query).collect();

    if modules.is_empty() {
        if output != "jsonl" {
            ceprintln!("<s,y>!</> No modules matched <s>{query}</>.");
        }
        return Ok(());
    }

    let width = modules
        .iter()
        .map(|module| module.name.len())
        .max()
        .unwrap_or_default();

    for module in modules {
        match output {
            "jsonl" => {
                let json = serde_json::to_string(module).map_err(|e| {
                    tracing::error!("failed to serialize module manifest: {e}");
                    EX_SOFTWARE
                })?;
                println!("{json}");
            },
            _ => {
                let name = format!("{:width$}", module.name);
                let summary = module.summary.as_deref().unwrap_or_default();
                cprintln!("<s,c>{name}</>  {summary}");
            },
        }
    }

    Ok(())
}
