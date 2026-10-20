// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use asimov_directory::fs::ResourcesDirectory;
use clientele::StandardOptions;
use tokio::io::{AsyncWriteExt, BufWriter};

mod urls;
pub use urls::ResourceListArgs;

/// Lists tracked resources, or uses module/relay listers for the given URLs.
///
/// Without URLs, prints the default collection in stored order, one URL per line.
/// A malformed file is reported after any preceding valid URLs have been printed.
pub async fn list(args: ResourceListArgs, flags: &StandardOptions) -> Result<(), BoxError> {
    if !args.urls.is_empty() {
        return urls::list(args, flags).await;
    }

    let mut tracked = super::resources()?
        .read_urls(ResourcesDirectory::DEFAULT_COLLECTION)
        .map_err(|e| super::report("list tracked resources", e))?;
    let mut stdout = BufWriter::new(tokio::io::stdout());
    let result = async {
        while let Some(url) = tracked
            .next_url()
            .await
            .map_err(|e| super::report("list tracked resources", e))?
        {
            stdout
                .write_all(url.as_bytes())
                .await
                .map_err(|e| super::report("write resource listing", e))?;
            stdout
                .write_all(b"\n")
                .await
                .map_err(|e| super::report("write resource listing", e))?;
        }
        Ok(())
    }
    .await;
    // Flush preceding URLs even when a later record fails validation.
    let flushed = stdout
        .flush()
        .await
        .map_err(|e| super::report("write resource listing", e));
    result.and(flushed)
}
