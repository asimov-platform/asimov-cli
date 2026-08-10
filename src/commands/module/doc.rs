// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError::*};
use color_print::ceprintln;
use core::error::Error;

pub async fn doc(
    module_name: impl AsRef<str>,
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    let module_name = module_name.as_ref().parse()?;
    let registry = asimov_registry::Registry::default();

    let readme = registry.read_readme(&module_name).await.map_err(|e| {
        tracing::error!("failed to read README for module `{module_name}`: {e}");
        EX_UNAVAILABLE
    })?;

    let Some(readme) = readme else {
        // A module installed before documentation shipped has no README, which
        // is a different problem from not having the module at all.
        return match registry.read_manifest(&module_name).await {
            Err(asimov_registry::error::ManifestError::NotInstalled) => {
                ceprintln!("<s,r>error:</> module `{module_name}` is not installed");
                ceprintln!(
                    "<s,dim>hint:</> Install it with: <s>asimov module install {module_name}</>"
                );
                Err(EX_UNAVAILABLE.into())
            },
            Err(e) => {
                tracing::error!("failed to read manifest for module `{module_name}`: {e}");
                Err(EX_UNAVAILABLE.into())
            },
            Ok(_) => {
                ceprintln!(
                    "<s,r>error:</> module `{module_name}` was installed without documentation"
                );
                ceprintln!(
                    "<s,dim>hint:</> Reinstall it to fetch the documentation: <s>asimov module uninstall {module_name} && asimov module install {module_name}</>"
                );
                ceprintln!(
                    "<s,dim>hint:</> Or read it online with: <s>asimov module browse {module_name}</>"
                );
                Err(EX_NOINPUT.into())
            },
        };
    };

    print!("{readme}");
    if !readme.ends_with('\n') {
        println!();
    }

    Ok(())
}
