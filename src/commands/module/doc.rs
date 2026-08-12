// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError::*};
use asimov_module::ModuleName;
use color_print::ceprintln;
use core::error::Error;

pub async fn doc(module_name: &ModuleName, _flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
    let registry = asimov_registry::Registry::default();

    let readme = match registry.read_readme(module_name).await {
        Ok(Some(readme)) => readme,
        Ok(None) => {
            ceprintln!("<s,r>error:</> module `{module_name}` was installed without documentation");
            ceprintln!(
                "<s,dim>hint:</> Reinstall it to fetch the documentation: <s>asimov module uninstall {module_name} && asimov module install {module_name}</>"
            );
            ceprintln!(
                "<s,dim>hint:</> Or try reading it online with: <s>asimov module browse {module_name}</>"
            );
            return Err(EX_NOINPUT.into());
        },
        Err(asimov_registry::error::ReadReadmeError::NotInstalled) => {
            ceprintln!("<s,r>error:</> module `{module_name}` is not installed");
            ceprintln!(
                "<s,dim>hint:</> Install it with: <s>asimov module install {module_name}</>"
            );
            return Err(EX_UNAVAILABLE.into());
        },
        Err(e) => {
            tracing::error!("failed to read README for module `{module_name}`: {e}");
            return Err(EX_UNAVAILABLE.into());
        },
    };

    print!("{readme}");
    if !readme.ends_with('\n') {
        println!();
    }

    Ok(())
}
