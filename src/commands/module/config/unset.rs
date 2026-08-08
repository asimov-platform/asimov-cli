// This is free and unencumbered software released into the public domain.

use super::open;
use clientele::StandardOptions;
use core::error::Error;

/// Removes stored values. Environment- and default-provided values are not
/// affected, since they are not stored here.
pub async fn unset(
    module_name: &str,
    keys: &[String],
    all: bool,
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    let module = open(module_name).await?;
    let variables = module.require_variables()?;

    let keys: Vec<&str> = if all {
        variables.iter().map(|var| var.name.as_str()).collect()
    } else {
        // validate every key first so a typo doesn't unset half the batch
        for key in keys {
            module.variable(key)?;
        }
        keys.iter().map(String::as_str).collect()
    };

    for key in keys {
        tokio::fs::remove_file(module.var_file(key))
            .await
            .or_else(|e| {
                if e.kind() == tokio::io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(e)
                }
            })
            .inspect_err(|e| {
                tracing::error!("failed to unset configuration variable `{key}`: {e}")
            })?;
    }

    module.set_permissions().await?;

    Ok(())
}
