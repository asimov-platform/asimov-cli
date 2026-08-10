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
        ceprintln!("<s,r>error:</> no documentation found for module `{module_name}`");
        ceprintln!(
            "<s,dim>hint:</> Check if the module is installed with: <s>asimov module list</>"
        );
        return Err(EX_UNAVAILABLE.into());
    };

    print!("{readme}");
    if !readme.ends_with('\n') {
        println!();
    }

    Ok(())
}
