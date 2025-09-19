// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
};
use asimov_env::paths::asimov_root;
use asimov_module::url::normalize_url;
use color_print::ceprintln;
use miette::Result;

pub async fn snap(input_urls: &[String], flags: &StandardOptions) -> Result<(), SysexitsError> {
    let registry = asimov_registry::Registry::default();

    let _enabled_modules = registry
        .enabled_modules()
        .await
        .map_err(|e| {
            ceprintln!("<s,r>error:</> unable to access enabled modules: {e}");
            match e {
                asimov_registry::error::EnabledModulesError::DirIo(_, err)
                    if err.kind() == std::io::ErrorKind::NotFound => {
                        ceprintln!("<s,dim>hint:</> There appears to be no installed modules.");
                        ceprintln!("<s,dim>hint:</> Modules may be discovered either on the site <u>https://asimov.directory/modules</>");
                        ceprintln!("<s,dim>hint:</> or on the GitHub organization <u>https://github.com/asimov-modules</>");
                        ceprintln!("<s,dim>hint:</> and installed with `asimov module install <<module>>`");
                    },
                _ => (),
            };
            EX_UNAVAILABLE
        })?;

    let storage =
        asimov_snapshot::storage::Fs::for_dir(asimov_root().join("snapshots")).map_err(|e| {
            ceprintln!("<s,r>error:</> failed to create snapshot storage: {e}");
            EX_UNAVAILABLE
        })?;

    let mut snapshotter = asimov_snapshot::Snapshotter::new(registry, storage, Default::default());

    for input_url in input_urls {
        let input_url = normalize_url(input_url).unwrap_or_else(|e| {
            if flags.verbose > 1 {
                ceprintln!(
                    "<s,y>warning:</> using given unmodified URL, normalization failed: {e}"
                );
            }
            input_url.clone()
        });
        snapshotter.snapshot(&input_url).await.map_err(|e| {
            ceprintln!("<s,r>error:</> failed to create snapshot URL `{input_url}`: {e}");
            EX_UNAVAILABLE
        })?;
    }

    Ok(())
}
