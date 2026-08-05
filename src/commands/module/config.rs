// This is free and unencumbered software released into the public domain.

use asimov_env::paths::asimov_root;
use clientele::{
    StandardOptions,
    SysexitsError::{self, *},
};
use color_print::ceprintln;
use core::error::Error;
use std::io::{BufRead, Write};

pub async fn config(
    module_name: &str,
    unset: bool,
    args: &[String],
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    let module_name = module_name.parse()?;
    let registry = asimov_registry::Registry::default();
    let manifest = registry
        .read_manifest(&module_name)
        .await
        .map_err(|e| {
            tracing::error!("failed to read manifest for module `{module_name}`: {e}");
            if let asimov_registry::error::ManifestError::NotInstalled = e {
                ceprintln!(
                    "<s,dim>hint:</> Check if the module is installed with: <s>asimov module list</>"
                );
            }
            EX_UNAVAILABLE
        })?
        .manifest;

    let conf_vars = manifest
        .config
        .as_ref()
        .map(|c| c.variables.as_slice())
        .unwrap_or_default();

    // Variable names become file names under the configuration directory;
    // reject anything that could escape it or hide files.
    let is_valid_name = |name: &str| {
        !name.is_empty()
            && !name.starts_with('.')
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    if let Some(var) = conf_vars.iter().find(|var| !is_valid_name(&var.name)) {
        ceprintln!(
            "<s,r>error:</> module <s>{module_name}</> declares an invalid configuration variable name: `{}`",
            var.name
        );
        return Err(EX_DATAERR.into());
    }

    if !conf_vars.is_empty() {
        let profile = "default"; // TODO

        let conf_dir = asimov_root()
            .join("configs")
            .join(profile)
            .join(module_name.as_str());

        if unset {
            let vars: Vec<&str> = if !args.is_empty() {
                for name in args {
                    if !conf_vars.iter().any(|var| var.name == *name) {
                        ceprintln!(
                            "<s,r>error:</> `{name}` is not the name of a configuration variable for <s>{module_name}</> module"
                        );
                        return Err(EX_USAGE.into());
                    }
                }
                args.iter().map(String::as_str).collect()
            } else {
                // unset all vars
                conf_vars.iter().map(|var| var.name.as_str()).collect()
            };

            for var in &vars {
                let var_file = conf_dir.join(var);
                tokio::fs::remove_file(&var_file)
                    .await
                    .or_else(|e| {
                        if e.kind() == tokio::io::ErrorKind::NotFound {
                            Ok(())
                        } else {
                            Err(e)
                        }
                    })
                    .inspect_err(|e| {
                        tracing::error!("failed to unset configuration variable `{var}`: {e}")
                    })?;
            }

            return Ok(()); // exit, without calling configurator
        }

        if args.is_empty() {
            // interactively prompt for each value in the config

            create_conf_dir(&conf_dir).await.inspect_err(|e| {
                tracing::error!(
                    "failed to create configuration directory for module `{module_name}`: {e}"
                )
            })?;

            let mut stdout = std::io::stdout().lock();
            let mut stdin = std::io::stdin().lock().lines();

            for var in conf_vars {
                let var_file = conf_dir.join(&var.name);

                let current_value = tokio::fs::read_to_string(&var_file).await.ok();

                let info_text = if current_value.is_some() {
                    "(press Enter to keep current)"
                } else if let Some(default_value) = &var.default_value {
                    &format!("(optional, default: `{default_value}`)")
                } else {
                    "(required)"
                };

                writeln!(&mut stdout, "Enter value for `{}` {info_text}", var.name)?;

                if let Some(current) = &current_value {
                    writeln!(&mut stdout, "Current value: `{}`", current.trim())?;
                }

                if let Some(desc) = &var.description {
                    writeln!(&mut stdout, "Description: {desc}")?;
                }

                write!(&mut stdout, "> ")?;
                stdout.flush()?;
                let value = stdin.next().ok_or(EX_NOINPUT)??;
                let value = value.trim();
                if value.is_empty() {
                    continue;
                }

                write_var_file(&var_file, value).await?;
            }

            writeln!(&mut stdout, "Configuration:")?;
            for var in conf_vars {
                match manifest.variable(&var.name, Some(profile)) {
                    Ok(val) => writeln!(&mut stdout, "\t{}: {}", var.name, val)?,
                    Err(e @ asimov_module::ReadVarError::UnconfiguredVar(_)) => {
                        ceprintln!("\t{}: <s,y>warn:</> {e}", var.name);
                    },
                    Err(e) => {
                        ceprintln!("\t{}: <s,r>error:</> {e}", var.name);
                    },
                }
            }
        } else if args.len() == 1 {
            // one arg, fetch the value

            let name = &args[0];
            match manifest.variable(name, Some(profile)) {
                Ok(value) => println!("{}", value.trim()),
                Err(asimov_module::ReadVarError::UnknownVar(_)) => {
                    ceprintln!("<s,r>error:</> unrecognized configuration variable key: `{name}`");
                    return Err(EX_USAGE.into());
                },
                Err(e @ asimov_module::ReadVarError::UnconfiguredVar(_)) => {
                    ceprintln!("<s,r>error:</> {e}");
                    return Err(EX_CONFIG.into());
                },
                Err(e) => {
                    ceprintln!("<s,r>error:</> {e}");
                    return Err(EX_IOERR.into());
                },
            }
        } else if args.len().is_multiple_of(2) {
            // pair(s) of (key,value), write into config file(s);
            // validate every key first so a typo doesn't apply half the batch

            for pair in args.chunks_exact(2) {
                let name = &pair[0];
                if !conf_vars.iter().any(|var| var.name == *name) {
                    ceprintln!(
                        "<s,r>error:</> `{name}` is not the name of a configuration variable for <s>{module_name}</> module"
                    );
                    return Err(EX_USAGE.into());
                }
            }

            create_conf_dir(&conf_dir).await.inspect_err(|e| {
                tracing::error!(
                    "failed to create configuration directory for module `{module_name}`: {e}"
                )
            })?;

            for pair in args.chunks_exact(2) {
                let [name, value] = pair else { unreachable!() };
                write_var_file(&conf_dir.join(name), value).await?;
            }
        } else {
            ceprintln!(
                "<s,r>error:</> invalid number of arguments: expected 0, 1, or key-value pairs (even count), got {}",
                args.len()
            );

            return Err(EX_USAGE.into());
        }
    }

    // A configurator is an interactive setup program; only run it in
    // interactive mode, never as a side effect of get/set/unset.
    if args.is_empty() && !unset {
        let configurator_name = format!("asimov-{module_name}-configurator");

        if manifest.provides.programs.contains(&configurator_name) {
            let conf_bin = asimov_root().join("libexec").join(&configurator_name);

            if !tokio::fs::try_exists(&conf_bin).await.unwrap_or(false) {
                ceprintln!(
                    "<s,r>error:</> module <s>{module_name}</> declares configurator `{configurator_name}`, but it is not installed"
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
    }

    Ok(())
}

async fn create_conf_dir(dir: &std::path::Path) -> tokio::io::Result<()> {
    let mut builder = tokio::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(dir).await
}

async fn write_var_file(path: &std::path::Path, value: &str) -> tokio::io::Result<()> {
    use tokio::io::AsyncWriteExt;
    let mut opts = tokio::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    opts.mode(0o600);
    let mut file = opts.open(path).await?;
    #[cfg(unix)]
    file.set_permissions(std::os::unix::fs::PermissionsExt::from_mode(0o600))
        .await?;
    file.write_all(value.as_bytes()).await
}
