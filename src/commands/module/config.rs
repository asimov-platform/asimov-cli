// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use asimov_env::paths::asimov_root;
use asimov_module::{ConfigurationVariable, ModuleManifest, ModuleName};
use clientele::{
    StandardOptions,
    SysexitsError::*,
    crates::clap::{Subcommand, builder::PossibleValuesParser},
};
use color_print::ceprintln;
use std::{path::PathBuf, string::String, vec::Vec};

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Show a module's configuration variables and their status
    #[clap(alias = "list")]
    Show {
        /// The name of the module
        name: ModuleName,

        /// Set the output format [default: cli] [possible values: cli, json]
        #[arg(value_name = "FORMAT", short = 'o', long)]
        #[arg(value_parser = PossibleValuesParser::new(["cli", "json"]), hide_possible_values = true)]
        output: Option<String>,
    },

    /// Print the value of a configuration variable
    Get {
        /// The name of the module
        name: ModuleName,

        /// The configuration variable to read
        key: String,

        /// Read the stored value only, ignoring the environment and any default
        #[arg(long)]
        stored: bool,
    },

    /// Set configuration variables
    Set {
        /// The name of the module
        name: ModuleName,

        /// The variables to set, as `key=value` pairs.
        /// With --stdin, a single bare key instead.
        #[arg(value_name = "KEY=VALUE", required_unless_present = "from_json")]
        assignments: Vec<String>,

        /// Read the value for a single key from standard input,
        /// keeping it out of the command line
        #[arg(long, conflicts_with = "from_json")]
        stdin: bool,

        /// Read a JSON object of key-value pairs from standard input
        #[arg(long, conflicts_with = "assignments")]
        from_json: bool,
    },

    /// Unset configuration variables
    Unset {
        /// The name of the module
        name: ModuleName,

        /// The configuration variables to unset
        #[arg(required_unless_present = "all")]
        keys: Vec<String>,

        /// Unset every configuration variable of the module
        #[arg(long, conflicts_with = "keys")]
        all: bool,
    },

    /// Configure a module interactively
    Setup {
        /// The name of the module
        name: ModuleName,
    },
}

impl ConfigCommand {
    pub async fn run(&self, flags: &StandardOptions) -> Result<(), BoxError> {
        use ConfigCommand::*;
        match self {
            Show { name, output } => {
                show::show(name, output.as_deref().unwrap_or("cli"), flags).await
            },
            Get { name, key, stored } => get::get(name, key, *stored, flags).await,
            Set {
                name,
                assignments,
                stdin,
                from_json,
            } => set::set(name, assignments, *stdin, *from_json, flags).await,
            Unset { name, keys, all } => unset::unset(name, keys, *all, flags).await,
            Setup { name } => setup::setup(name, flags).await,
        }
    }
}

mod get;
mod set;
mod setup;
mod show;
mod unset;

/// Stands in for secret values, which are never displayed unless requested
/// explicitly by name.
pub(super) const MASK: &str = "******";

/// An installed module together with the location of its configuration.
pub(super) struct Module {
    pub name: ModuleName,
    pub manifest: ModuleManifest,
    pub profile: &'static str,
    pub conf_dir: PathBuf,
}

/// Reads the manifest of an installed module, rejecting manifests whose
/// variable names cannot be used as file names.
pub(super) async fn open(module_name: &ModuleName) -> Result<Module, BoxError> {
    let manifest = asimov_registry::Registry::default()
        .read_manifest(module_name)
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

    // Variable names become file names under the configuration directory;
    // reject anything that could escape it or hide files.
    let is_valid_name = |name: &str| {
        !name.is_empty()
            && !name.starts_with('.')
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };

    let variables = manifest
        .config
        .as_ref()
        .map(|c| c.variables.as_slice())
        .unwrap_or_default();

    if let Some(var) = variables.iter().find(|var| !is_valid_name(&var.name)) {
        ceprintln!(
            "<s,r>error:</> module <s>{module_name}</> declares an invalid configuration variable name: `{}`",
            var.name
        );
        return Err(EX_DATAERR.into());
    }

    let profile = "default"; // TODO
    let conf_dir = asimov_root()
        .join("configs")
        .join(profile)
        .join(module_name.as_str());

    Ok(Module {
        name: module_name.clone(),
        manifest,
        profile,
        conf_dir,
    })
}

impl Module {
    pub fn variables(&self) -> &[ConfigurationVariable] {
        self.manifest
            .config
            .as_ref()
            .map(|c| c.variables.as_slice())
            .unwrap_or_default()
    }

    /// Looks up a declared variable, reporting unknown keys as a usage error.
    pub fn variable(&self, key: &str) -> Result<&ConfigurationVariable, BoxError> {
        self.variables()
            .iter()
            .find(|var| var.name == key)
            .ok_or_else(|| {
                ceprintln!(
                    "<s,r>error:</> `{key}` is not the name of a configuration variable for <s>{}</> module",
                    self.name
                );
                EX_USAGE.into()
            })
    }

    /// Reports modules that declare no configuration variables as a usage
    /// error, so that operating on their variables is never silently a no-op.
    pub fn require_variables(&self) -> Result<&[ConfigurationVariable], BoxError> {
        let variables = self.variables();
        if variables.is_empty() {
            ceprintln!(
                "<s,r>error:</> module <s>{}</> has no configuration variables",
                self.name
            );
            return Err(EX_USAGE.into());
        }
        Ok(variables)
    }

    pub fn var_file(&self, key: &str) -> PathBuf {
        self.conf_dir.join(key)
    }

    /// Where the effective value of a variable comes from, in the same
    /// precedence the SDK resolves them: environment, then stored, then default.
    pub async fn source(&self, var: &ConfigurationVariable) -> Source {
        if let Some(env_name) = var.environment.as_deref()
            && std::env::var(env_name).is_ok()
        {
            return Source::Environment;
        }
        if tokio::fs::try_exists(self.var_file(&var.name))
            .await
            .unwrap_or(false)
        {
            return Source::Stored;
        }
        if var.default_value.is_some() {
            return Source::Default;
        }
        Source::Unset
    }

    pub async fn create_conf_dir(&self) -> tokio::io::Result<()> {
        tokio::fs::create_dir_all(&self.conf_dir)
            .await
            .inspect_err(|e| {
                tracing::error!(
                    "failed to create configuration directory for module `{}`: {e}",
                    self.name
                )
            })
    }

    #[cfg(not(unix))]
    pub async fn set_permissions(&self) -> tokio::io::Result<()> {
        Ok(())
    }

    #[cfg(unix)]
    pub async fn set_permissions(&self) -> tokio::io::Result<()> {
        async {
            use std::os::unix::fs::PermissionsExt;

            let metadata = match tokio::fs::symlink_metadata(&self.conf_dir).await {
                Ok(metadata) => metadata,
                Err(e) if e.kind() == tokio::io::ErrorKind::NotFound => return Ok(()),
                Err(e) => return Err(e),
            };
            if metadata.is_symlink() {
                return Ok(());
            }

            let mut directories = vec![self.conf_dir.clone()];
            while let Some(directory) = directories.pop() {
                tokio::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
                    .await?;

                let mut entries = tokio::fs::read_dir(&directory).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();
                    let metadata = tokio::fs::symlink_metadata(&path).await?;
                    if metadata.is_dir() {
                        directories.push(path);
                    } else if metadata.is_file() {
                        tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                            .await?;
                    }
                }
            }

            Ok(())
        }
        .await
        .inspect_err(|e| {
            tracing::error!(
                "failed to set configuration permissions for module `{}`: {e}",
                self.name
            )
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Source {
    Environment,
    Stored,
    Default,
    Unset,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Environment => "environment",
            Source::Stored => "stored",
            Source::Default => "default",
            Source::Unset => "unset",
        }
    }
}

/// Prompts for one value on the terminal, hiding what is typed when the value
/// is secret. Reads via the terminal rather than stdin, so that a buffered read
/// cannot consume bytes a later prompt needs.
pub(super) fn prompt_for_value(prompt: String, secret: bool) -> Result<String, BoxError> {
    let input = if secret {
        dialoguer::Password::new()
            .with_prompt(prompt)
            .allow_empty_password(true)
            .interact()
    } else {
        dialoguer::Input::<String>::new()
            .with_prompt(prompt)
            .allow_empty(true)
            .interact_text()
    };

    input.map_err(|e| {
        tracing::error!("failed to read a value from the terminal: {e}");
        EX_IOERR.into()
    })
}
