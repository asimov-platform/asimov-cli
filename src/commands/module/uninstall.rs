// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
};
use color_print::cprintln;

pub async fn uninstall(
    module_names: &Vec<String>,
    flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let installer = asimov_installer::Installer::default();
    for module_name in module_names {
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
