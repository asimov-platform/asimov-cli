// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use asimov_directory::fs::ResourcesDirectory;
use clientele::StandardOptions;
use std::io::Write;

mod urls;
pub use urls::ResourceListArgs;

/// Lists tracked resources, or uses module/relay listers for the given URLs.
///
/// Without URLs, prints the default collection in stored order, one URL per line.
pub async fn list(args: ResourceListArgs, flags: &StandardOptions) -> Result<(), BoxError> {
    if !args.urls.is_empty() {
        return urls::list(args, flags).await;
    }

    let tracked = super::resources()?
        .read_urls(ResourcesDirectory::DEFAULT_COLLECTION)
        .map_err(|e| super::report("list tracked resources", e))?;
    write!(std::io::stdout().lock(), "{tracked}")
        .map_err(|e| super::report("write resource listing", e))?;
    Ok(())
}
