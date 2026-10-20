// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::{string::String, vec::Vec};
use asimov_directory::fs::ResourcesDirectory;
use clientele::StandardOptions;

/// Appends new URLs to the default tracked collection in input order.
pub async fn track(urls: Vec<String>, _flags: &StandardOptions) -> Result<(), BoxError> {
    let urls = super::parse_urls(urls)?;
    super::resources()?
        .append_urls(ResourcesDirectory::DEFAULT_COLLECTION, &urls)
        .await
        .map_err(|e| super::report("track resources", e))?;
    Ok(())
}
