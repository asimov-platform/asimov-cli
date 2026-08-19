// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, SysexitsError::*};
use asimov_module::ModuleName;

pub async fn link(module_name: ModuleName, _flags: &StandardOptions) -> Result<(), BoxError> {
    let registry = asimov_registry::Registry::default();

    let manifest = registry
        .read_manifest(&module_name)
        .await
        .map_err(|e| {
            tracing::error!("failed to read module manifest: {e}");
            EX_UNAVAILABLE
        })?
        .manifest;

    let mut links = manifest.links;
    crate::sort_links(&manifest.name, &mut links);

    for link in links {
        println!("{link}");
    }

    Ok(())
}
