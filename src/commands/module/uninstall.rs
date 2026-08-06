// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError::*};
use color_print::cprintln;
use core::error::Error;

pub async fn uninstall(
    module_names: &Vec<String>,
    flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    let installer = asimov_installer::Installer::default();
    for module_name in module_names {
        let module_name = module_name.parse()?;

        if flags.verbose > 1 {
            cprintln!("<s,c>»</> Uninstalling the module <s>{module_name}</>...");
        }

        installer
            .uninstall_module(&module_name)
            .await
            .map_err(|e| {
                tracing::error!("failed to uninstall module `{module_name}`: {e}");
                EX_UNAVAILABLE
            })?;

        if flags.verbose > 0 {
            cprintln!("<s,g>✓</> Uninstalled the module <s>{module_name}</>.");
        }
    }
    Ok(())
}
