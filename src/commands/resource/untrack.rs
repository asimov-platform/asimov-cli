// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::{string::String, vec::Vec};
use asimov_directory::fs::ResourcesDirectory;
use clientele::StandardOptions;

/// Removes URLs from the default collection, silently ignoring absent URLs.
pub async fn untrack(urls: Vec<String>, _flags: &StandardOptions) -> Result<(), BoxError> {
    let urls = super::parse_urls(urls)?;
    super::resources()?
        .remove_urls(ResourcesDirectory::DEFAULT_COLLECTION, &urls)
        .await
        .map_err(|e| super::report("untrack resources", e))?;
    Ok(())
}
