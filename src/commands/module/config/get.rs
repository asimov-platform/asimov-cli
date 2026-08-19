// This is free and unencumbered software released into the public domain.

use super::open;
use crate::BoxError;
use asimov_module::ModuleName;
use clientele::{StandardOptions, SysexitsError::*};
use color_print::ceprintln;

/// Prints the effective value of a configuration variable, resolved the same
/// way modules resolve it: environment, then stored value, then default.
pub async fn get(
    module_name: &ModuleName,
    key: &str,
    stored: bool,
    _flags: &StandardOptions,
) -> Result<(), BoxError> {
    let module = open(module_name).await?;
    module.require_variables()?;
    module.variable(key)?;

    if stored {
        return match tokio::fs::read_to_string(module.var_file(key)).await {
            Ok(value) => {
                println!("{}", value.trim());
                Ok(())
            },
            Err(e) if e.kind() == tokio::io::ErrorKind::NotFound => {
                ceprintln!("<s,r>error:</> no value is stored for variable `{key}`");
                Err(EX_CONFIG.into())
            },
            Err(e) => {
                ceprintln!("<s,r>error:</> failed to read variable `{key}`: {e}");
                Err(EX_IOERR.into())
            },
        };
    }

    match module.manifest.variable(key, Some(module.profile)) {
        Ok(value) => {
            println!("{}", value.trim());
            Ok(())
        },
        Err(e @ asimov_module::ReadVarError::UnconfiguredVar(_)) => {
            ceprintln!("<s,r>error:</> {e}");
            Err(EX_CONFIG.into())
        },
        Err(e) => {
            ceprintln!("<s,r>error:</> {e}");
            Err(EX_IOERR.into())
        },
    }
}
