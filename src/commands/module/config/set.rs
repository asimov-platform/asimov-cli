// This is free and unencumbered software released into the public domain.

use super::open;
use clientele::{StandardOptions, SysexitsError::*};
use color_print::ceprintln;
use core::error::Error;
use std::io::Read;

/// Stores values for one or more configuration variables.
pub async fn set(
    module_name: &str,
    assignments: &[String],
    stdin: bool,
    from_json: bool,
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    let module = open(module_name).await?;
    module.require_variables()?;

    // collect and validate everything first so a typo doesn't apply half the batch
    let pairs: Vec<(String, String)> = if from_json {
        let input: serde_json::Map<String, serde_json::Value> =
            serde_json::from_reader(std::io::stdin().lock()).map_err(|e| {
                ceprintln!("<s,r>error:</> failed to read a JSON object from standard input: {e}");
                EX_DATAERR
            })?;

        input
            .into_iter()
            .map(|(key, value)| match value {
                serde_json::Value::String(value) => Ok((key, value)),
                _ => {
                    ceprintln!("<s,r>error:</> the value of `{key}` is not a string");
                    Err(EX_DATAERR.into())
                },
            })
            .collect::<Result<_, Box<dyn Error>>>()?
    } else if stdin {
        let [key] = assignments else {
            ceprintln!(
                "<s,r>error:</> expected exactly one variable name with --stdin, got {}",
                assignments.len()
            );
            return Err(EX_USAGE.into());
        };

        let mut value = String::new();
        std::io::stdin().lock().read_to_string(&mut value)?;

        // a trailing newline is an artifact of how the value was piped in
        let value = value
            .strip_suffix('\n')
            .map(|value| value.strip_suffix('\r').unwrap_or(value))
            .unwrap_or(&value);

        vec![(key.clone(), value.to_string())]
    } else {
        assignments
            .iter()
            .map(|assignment| {
                let Some((key, value)) = assignment.split_once('=') else {
                    ceprintln!("<s,r>error:</> expected `key=value`, got `{assignment}`");
                    ceprintln!(
                        "<s,dim>hint:</> Read a value instead with: <s>asimov module config get {module_name} {assignment}</>"
                    );
                    return Err(EX_USAGE.into());
                };
                Ok((key.to_string(), value.to_string()))
            })
            .collect::<Result<_, Box<dyn Error>>>()?
    };

    for (key, _) in &pairs {
        module.variable(key)?;
    }

    module.create_conf_dir().await?;

    for (key, value) in &pairs {
        tokio::fs::write(module.var_file(key), value).await?;
    }

    module.set_permissions().await?;

    Ok(())
}
