// This is free and unencumbered software released into the public domain.

use super::{MASK, open, prompt_for_value};
use crate::BoxError;
use asimov_env::paths::asimov_root;
use asimov_module::ModuleName;
use clientele::{
    StandardOptions,
    SysexitsError::{self, *},
};
use color_print::ceprintln;
use std::io::{IsTerminal, Write};

/// Configures a module interactively: prompts for each declared variable, then
/// hands over to the module's own configurator, if it provides one.
pub async fn setup(module_name: &ModuleName, _flags: &StandardOptions) -> Result<(), BoxError> {
    let module = open(module_name).await?;
    let variables = module.variables();

    let configurator_name = format!("asimov-{}-configurator", module.name);
    let has_configurator = module
        .manifest
        .provides
        .programs
        .contains(&configurator_name);

    if variables.is_empty() && !has_configurator {
        ceprintln!(
            "<s,dim>note:</> module <s>{}</> has no configuration",
            module.name
        );
        return Ok(());
    }

    if !std::io::stdin().is_terminal() {
        ceprintln!("<s,r>error:</> interactive configuration requires a terminal");
        ceprintln!(
            "<s,dim>hint:</> Set values non-interactively with: <s>asimov module config set {module_name} KEY=VALUE</>"
        );
        return Err(EX_UNAVAILABLE.into());
    }

    if !variables.is_empty() {
        module.create_conf_dir().await?;
        module.set_permissions().await?;

        // prompts go to stderr so stdout stays clean for actual output
        let mut stderr = std::io::stderr().lock();

        for var in variables {
            let var_file = module.var_file(&var.name);
            let current_value = tokio::fs::read_to_string(&var_file).await.ok();

            let info_text = if current_value.is_some() {
                "(press Enter to keep current)"
            } else if var.secret {
                "(secret, input is hidden)"
            } else if let Some(default_value) = &var.default_value {
                &format!("(optional, default: `{default_value}`)")
            } else if var.is_required() {
                "(required)"
            } else {
                "(optional)"
            };

            if let Some(desc) = &var.description {
                writeln!(&mut stderr, "{desc}")?;
            }

            if let Some(current) = &current_value {
                if var.secret {
                    writeln!(&mut stderr, "Current value: {MASK}")?;
                } else {
                    writeln!(&mut stderr, "Current value: `{}`", current.trim())?;
                }
            }

            let value = prompt_for_value(
                format!("Enter value for `{}` {info_text}", var.name),
                var.secret,
            )?;

            let value = value.trim();
            if value.is_empty() {
                continue;
            }

            tokio::fs::write(&var_file, value).await?;
        }

        let mut stdout = std::io::stdout().lock();
        writeln!(&mut stdout, "Configuration:")?;
        for var in variables {
            match module.manifest.variable(&var.name, Some(module.profile)) {
                Ok(_) if var.secret => writeln!(&mut stdout, "\t{}: {MASK}", var.name)?,
                Ok(val) => writeln!(&mut stdout, "\t{}: {}", var.name, val)?,
                Err(e @ asimov_module::ReadVarError::UnconfiguredVar(_)) => {
                    ceprintln!("\t{}: <s,y>warn:</> {e}", var.name);
                },
                Err(e) => {
                    ceprintln!("\t{}: <s,r>error:</> {e}", var.name);
                },
            }
        }
    }

    if has_configurator {
        let conf_bin = asimov_root().join("libexec").join(&configurator_name);

        if !tokio::fs::try_exists(&conf_bin).await.unwrap_or(false) {
            ceprintln!(
                "<s,r>error:</> module <s>{}</> declares configurator `{configurator_name}`, but it is not installed",
                module.name
            );
            return Err(EX_UNAVAILABLE.into());
        }

        let status = std::process::Command::new(&conf_bin)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .inspect_err(|e| tracing::error!("failed to execute configurator: {e}"))?;

        if !status.success() {
            ceprintln!("<s,r>error:</> configurator `{configurator_name}` failed: {status}");
            return Err(SysexitsError::try_from(status)
                .unwrap_or(EX_SOFTWARE)
                .into());
        }
    }

    module.set_permissions().await?;

    Ok(())
}
