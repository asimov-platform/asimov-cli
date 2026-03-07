// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
};

pub async fn browse(
    module_name: impl AsRef<str>,
    _flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let module_name = module_name.as_ref();
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

    if let Some(link) = links.first() {
        open::that(link).inspect_err(|e| tracing::error!("failed to open URL `{link}`: {e}"))?;
        return Ok(());
    }

    eprintln!("unable to browse module: {module_name}");
    Err(EX_UNAVAILABLE)
}
