// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError::*};
use core::error::Error;

pub async fn link(
    module_name: impl AsRef<str>,
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    let module_name = module_name.as_ref().parse()?;
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
