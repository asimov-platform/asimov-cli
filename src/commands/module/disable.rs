// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
};
use color_print::cprintln;

pub async fn disable(
    module_names: &Vec<String>,
    flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let registry = asimov_registry::Registry::default();
    for module_name in module_names {
        if flags.verbose > 1 {
            cprintln!("<s,c>»</> Disabling module <s>{module_name}</>...");
        }

        registry.disable_module(&module_name).await.map_err(|e| {
            tracing::error!("failed to disable module `{module_name}`: {e}");
            EX_UNAVAILABLE
        })?;

        if flags.verbose > 0 {
            cprintln!("<s,g>✓</> Disabled module <s>{module_name}</>.");
        }
    }
    Ok(())
}
