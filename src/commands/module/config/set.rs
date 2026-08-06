// This is free and unencumbered software released into the public domain.

use super::{open, write_var_file};
use clientele::{StandardOptions, SysexitsError::*};
use color_print::ceprintln;
use core::error::Error;

/// Stores values for one or more configuration variables.
pub async fn set(
    module_name: &str,
    assignments: &[String],
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    let module = open(module_name).await?;
    module.require_variables()?;

    // validate every assignment first so a typo doesn't apply half the batch
    let mut pairs = Vec::with_capacity(assignments.len());
    for assignment in assignments {
        let Some((key, value)) = assignment.split_once('=') else {
            ceprintln!("<s,r>error:</> expected `key=value`, got `{assignment}`");
            ceprintln!(
                "<s,dim>hint:</> Read a value instead with: <s>asimov module config get {module_name} {assignment}</>"
            );
            return Err(EX_USAGE.into());
        };
        module.variable(key)?;
        pairs.push((key, value));
    }

    module.create_conf_dir().await?;

    for (key, value) in pairs {
        write_var_file(&module.var_file(key), value).await?;
    }

    Ok(())
}
