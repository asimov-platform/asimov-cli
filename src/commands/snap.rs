// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
    shared::{normalize_url},
};
use asimov_env::paths::asimov_root;
use asimov_module::{ModuleManifest, resolve::Resolver};
use color_print::ceprintln;
use miette::Result;

pub async fn snap(input_urls: &[String], flags: &StandardOptions) -> Result<(), SysexitsError> {
    let registry = asimov_registry::Registry::default();

    let enabled_modules = registry
        .enabled_modules()
        .await
        .map_err(|e| {
            ceprintln!("<s,r>error:</> unable to access enabled modules: {e}");
            EX_UNAVAILABLE
        })?
        .into_iter()
        .map(|manifest| manifest.manifest);

    let resolver = Resolver::try_from_iter(enabled_modules).map_err(|e| {
        ceprintln!("<s,r>error:</> failed to build resolver: {e}");
        EX_UNAVAILABLE
    })?;

    let storage =
        asimov_snapshot::storage::Fs::for_dir(asimov_root().join("snapshots")).map_err(|e| {
            ceprintln!("<s,r>error:</> failed to create snapshot storage: {e}");
            EX_UNAVAILABLE
        })?;

    let mut snapshotter =
        asimov_snapshot::Snapshotter::new(resolver, storage, asimov_snapshot::Options::default());

    for input_url in input_urls {
        let input_url = normalize_url(input_url);
        snapshotter.snapshot(&input_url).await.map_err(|e| {
            ceprintln!("<s,r>error:</> failed to create snapshot URL `{input_url}`: {e}");
            EX_UNAVAILABLE
        })?;
    }

    Ok(())
}
