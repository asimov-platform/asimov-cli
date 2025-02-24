// This is free and unencumbered software released into the public domain.

use std::{fs::ReadDir, path::PathBuf};

pub type Result<T> = std::io::Result<T>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ExternalCommand {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ExternalCommands {
    commands: Vec<ExternalCommand>,
}

impl ExternalCommands {
    pub fn collect(prefix: &str, level: usize) -> Result<ExternalCommands> {
        let commands = Self::collect_internal(prefix, level)?;
        Ok(ExternalCommands { commands })
    }

    fn collect_internal(prefix: &str, level: usize) -> Result<Vec<ExternalCommand>> {
        let Some(paths) = std::env::var_os("PATH") else {
            // PATH variable is not set.
            return Ok(vec![]);
        };

        let mut result = vec![];
        for path in std::env::split_paths(&paths) {
            let Ok(dir) = std::fs::read_dir(path) else {
                continue;
            };

            let commands = Self::collect_in_dir(prefix, dir)?
                .into_iter()
                // Construct ExternalCommand.
                .flat_map(|path| {
                    let name = path
                        .file_stem()?
                        .to_string_lossy()
                        .trim_start_matches(prefix)
                        .to_string();

                    Some(ExternalCommand { name, path })
                })
                // Respect level.
                .filter(|cmd| {
                    let count = cmd.name.chars().filter(|&c| c == '-').count();
                    count < level
                });

            result.extend(commands);
        }

        Ok(result)
    }
}

impl ExternalCommands {
    pub fn iter(&self) -> impl Iterator<Item = &ExternalCommand> {
        self.commands.iter()
    }

    pub fn find(&self, name: &str) -> Option<&ExternalCommand> {
        self.commands.iter().find(|command| command.name == name)
    }
}

#[cfg(unix)]
impl ExternalCommands {
    fn is_executable_metadata(metadata: std::fs::Metadata) -> bool {
        use std::os::unix::prelude::*;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }

    fn collect_in_dir(prefix: &str, dir: ReadDir) -> Result<Vec<PathBuf>> {
        let mut result = vec![];

        for entry in dir {
            let Ok(entry) = entry else {
                // invalid entry.
                continue;
            };

            let file_name = entry.file_name();
            let Some(entry_name) = file_name.to_str() else {
                // skip files with invalid names.
                continue;
            };

            if !entry_name.starts_with(prefix) {
                // skip non-matching files.
                continue;
            }

            if entry_name.starts_with(".")
                || entry_name.ends_with("~")
                || !entry_name.starts_with(prefix)
            {
                // skip hidden and backup files.
                continue;
            }

            let Ok(metadata) = entry.metadata() else {
                // couldn't get metadata.
                continue;
            };

            if !Self::is_executable_metadata(metadata) {
                // skip non-executable files.
                continue;
            }

            result.push(entry.path());
        }

        Ok(result)
    }
}

#[cfg(windows)]
impl ExternalCommands {
    fn collect_in_dir(prefix: &str, dir: ReadDir) -> Result<Vec<PathBuf>> {
        use std::env::consts::EXE_EXTENSION;
        use std::os::windows::fs::MetadataExt;
        use std::path::PathBuf;

        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x00000002;

        // FIXME: Currently we don't handle the case when
        // there are multiple files with the same name,
        // but different extensions. This is a bug.
        static BAT_EXTENSION: &str = "bat";
        static CMD_EXTENSION: &str = "cmd";
        static ALLOWED_EXTENSIONS: [&str; 3] = [BAT_EXTENSION, CMD_EXTENSION, EXE_EXTENSION];

        let mut result = vec![];

        for entry in dir {
            let Ok(entry) = entry else {
                // invalid entry.
                continue;
            };

            let file_name = entry.file_name();
            let Some(entry_name) = file_name.to_str() else {
                // skip files with invalid names.
                continue;
            };

            if !entry_name.starts_with(prefix) {
                // skip non-matching files.
                continue;
            }

            let entry_path = entry.path();
            let Some(entry_ext) = entry_path.extension().and_then(|ext| ext.to_str()) else {
                // skip files without extensions.
                continue;
            };

            let entry_ext = entry_ext.to_lowercase();
            if !ALLOWED_EXTENSIONS.contains(&entry_ext.as_str()) {
                // skip non-executable files
                continue;
            }

            let metadata = entry.metadata()?;
            let is_hidden = metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0;
            if is_hidden {
                // skip hidden files
                continue;
            }

            result.push(entry_path);
        }

        Ok(result)
    }
}
