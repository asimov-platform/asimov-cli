// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::{string::String, vec::Vec};
use asimov_directory::fs::ResourcesDirectory;
use clientele::StandardOptions;

/// Removes IRIs from the default collection, silently ignoring absent IRIs.
pub async fn untrack(urls: Vec<String>, _flags: &StandardOptions) -> Result<(), BoxError> {
    let uris = super::parse_uris(urls)?;
    super::resources()?
        .remove_uris(ResourcesDirectory::DEFAULT_COLLECTION, &uris)
        .await
        .map_err(|e| super::report("untrack resources", e))?;
    Ok(())
}
