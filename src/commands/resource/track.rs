// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::{string::String, vec::Vec};
use asimov_directory::fs::ResourcesDirectory;
use clientele::StandardOptions;

/// Appends new IRIs to the default tracked collection in input order.
pub async fn track(urls: Vec<String>, _flags: &StandardOptions) -> Result<(), BoxError> {
    let uris = super::parse_uris(urls)?;
    super::resources()?
        .append_uris(ResourcesDirectory::DEFAULT_COLLECTION, &uris)
        .await
        .map_err(|e| super::report("track resources", e))?;
    Ok(())
}
