// This is free and unencumbered software released into the public domain.

use asimov_runner::PrompterOptions;
use color_print::ceprintln;

use crate::{
    StandardOptions,
    SysexitsError::{self, *},
    shared,
};

pub async fn ask(
    input: impl AsRef<str>,
    module_filter: Option<&str>,
    model: Option<&str>,
    flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let registry = asimov_registry::Registry::default();
    let modules = shared::installed_modules(&registry, Some("prompter")).await?;

    let module = if let Some(filter) = module_filter {
        let module = modules.iter().find(|m| m.name == filter).ok_or_else(|| {
            ceprintln!(
                "<s,r>error:</> failed to find a module named `{filter}` that provides a prompter"
            );
            EX_SOFTWARE
        })?;

        if !registry
            .is_module_enabled(&module.name)
            .await
            .map_err(|e| {
                ceprintln!(
                    "<s,r>error:</> error while checking whether module `{}` is enabled: {e}",
                    module.name
                );
                EX_IOERR
            })?
        {
            ceprintln!(
                "<s,r>error:</> module <s>{}</> is not enabled.",
                module.name
            );
            ceprintln!(
                "<s,dim>hint:</> It can be enabled with: <s>asimov module enable {}</>",
                module.name
            );
            return Err(EX_UNAVAILABLE);
        } else {
            module
        }
    } else {
        let mut iter = modules.iter();
        loop {
            let module = iter.next().ok_or_else(|| {
                ceprintln!("<s,r>error:</> failed to find a module for prompting");
                    let module_count = modules.len();
                    if module_count > 0 {
                        if module_count == 1 {
                            ceprintln!("<s,dim>hint:</> Found <s>{module_count}</> installed module that provides a prompter but is disabled.");
                        } else {
                            ceprintln!("<s,dim>hint:</> Found <s>{module_count}</> installed modules that provide a prompter but are disabled.");
                        }
                        ceprintln!("<s,dim>hint:</> A module can be enabled with: <s>asimov module enable <<module>></>");
                        ceprintln!("<s,dim>hint:</> Available modules:");
                        for module in &modules {
                            ceprintln!("<s,dim>hint:</>\t<s>{}</>", module.name);
                        }
                    }
                EX_UNAVAILABLE
            })?;

            if !registry
                .is_module_enabled(&module.name)
                .await
                .map_err(|e| {
                    ceprintln!(
                        "<s,r>error:</> error while checking whether module `{}` is enabled: {e}",
                        module.name
                    );
                    EX_IOERR
                })?
            {
                continue;
            }

            break module;
        }
    };

    let program = format!("asimov-{}-prompter", module.name);

    let mut prompter = asimov_runner::Prompter::new(
        program,
        input.as_ref().into(),
        asimov_runner::Output::Captured,
        PrompterOptions::builder()
            .maybe_other(flags.debug.then_some("--debug"))
            .maybe_model(model)
            .build(),
    );

    let result = prompter.execute().await.expect("should execute prompter");

    print!("{result}");

    Ok(())
}
