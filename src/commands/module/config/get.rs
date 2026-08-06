// This is free and unencumbered software released into the public domain.

use super::open;
use clientele::{StandardOptions, SysexitsError::*};
use color_print::ceprintln;
use core::error::Error;

/// Prints the effective value of a configuration variable, resolved the same
/// way modules resolve it: environment, then stored value, then default.
pub async fn get(
    module_name: &str,
    key: &str,
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    let module = open(module_name).await?;
    module.require_variables()?;
    module.variable(key)?;

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
