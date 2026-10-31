// This is free and unencumbered software released into the public domain.

use crate::{BoxError, shared};
use asimov_directory::fs::ResourcesDirectory;
use clientele::StandardOptions;
use std::io::{BufWriter, Write};

mod urls;
pub use urls::ResourceListArgs;

/// Lists tracked resources, or uses module/relay listers for the given URLs.
///
/// Without URLs, prints the default collection in stored order, one IRI per line.
/// A malformed file is reported after any preceding valid IRIs have been printed.
pub async fn list(args: ResourceListArgs, flags: &StandardOptions) -> Result<(), BoxError> {
    if !args.urls.is_empty() {
        return urls::list(args, flags).await;
    }

    let mut tracked = super::resources()?
        .read_uris(ResourcesDirectory::DEFAULT_COLLECTION)
        .map_err(|e| super::report("list tracked resources", e))?;
    let mut stdout = BufWriter::new(shared::stdout());
    let result = async {
        while let Some(uri) = tracked
            .next_uri()
            .await
            .map_err(|e| super::report("list tracked resources", e))?
        {
            writeln!(stdout, "{uri}").map_err(|e| super::report("write resource listing", e))?;
        }
        Ok(())
    }
    .await;
    // Flush preceding IRIs even when a later record fails validation.
    let flushed = stdout
        .flush()
        .map_err(|e| super::report("write resource listing", e));
    result.and(flushed)
}
