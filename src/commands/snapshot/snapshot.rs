// This is free and unencumbered software released into the public domain.

use asimov_env::paths::asimov_root;
use asimov_module::normalization::normalize_url;
use asimov_registry::Registry;
use asimov_snapshot::{Options, Snapshotter};
use clientele::{StandardOptions, SysexitsError};

pub async fn snapshot(urls: &[String], _flags: &StandardOptions) -> Result<(), SysexitsError> {
    let storage = asimov_snapshot::storage::Fs::for_dir(asimov_root().join("snapshots"))?;
    let mut ss = Snapshotter::new(Registry::default(), storage, Options::default());

    for url in urls {
        let url = normalize_url(url).unwrap_or_else(|e| {
            tracing::error!(
                url,
                "proceeding with given unmodified URL, normalization failed: {e}"
            );
            url.into()
        });

        ss.snapshot(&url)
            .await
            .inspect_err(|e| tracing::error!("failed to snapshot the resource `{url}`: {e}"))?;
    }
    Ok(())
}
