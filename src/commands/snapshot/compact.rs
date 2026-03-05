// This is free and unencumbered software released into the public domain.

use asimov_env::paths::asimov_root;
use asimov_module::normalization::normalize_url;
use asimov_registry::Registry;
use asimov_snapshot::{Options, Snapshotter};
use clientele::{StandardOptions, SysexitsError};

pub async fn compact(urls: &[String], _flags: &StandardOptions) -> Result<(), SysexitsError> {
    let storage = asimov_snapshot::storage::Fs::for_dir(asimov_root().join("snapshots"))?;
    let ss = Snapshotter::new(Registry::default(), storage, Options::default());

    let urls: Vec<String> = if !urls.is_empty() {
        urls.iter()
            .map(|url| {
                normalize_url(url).unwrap_or_else(|e| {
                    tracing::error!(
                        url,
                        "proceeding with given unmodified URL, normalization failed: {e}"
                    );
                    url.clone()
                })
            })
            .collect()
    } else {
        ss.list()
            .await
            .inspect_err(|e| tracing::error!("failed to read previously snapshotted URLs: {e}"))?
            .into_iter()
            .map(|(url, _)| url)
            .collect()
    };

    for url in urls {
        ss.compact(&url)
            .await
            .inspect_err(|e| tracing::error!("failed to compact snapshots for `{url}`: {e}"))?;
    }
    Ok(())
}
