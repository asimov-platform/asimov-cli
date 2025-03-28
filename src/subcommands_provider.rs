// This is free and unencumbered software released into the public domain.

use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Subcommand {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SubcommandsProvider {
    commands: Vec<Subcommand>,
}

impl SubcommandsProvider {
    pub fn collect(prefix: &str, level: usize) -> SubcommandsProvider {
        let commands = Self::collect_commands(prefix)
            .into_iter()
            // Construct ExternalCommand.
            .flat_map(|path| {
                let name = path
                    .file_stem()?
                    .to_string_lossy()
                    .trim_start_matches(prefix)
                    .to_string();

                Some(Subcommand { name, path })
            })
            // Respect level.
            .filter(|cmd| {
                let count = cmd.name.chars().filter(|&c| c == '-').count();
                count < level
            })
            .collect();

        SubcommandsProvider { commands }
    }

    pub fn find(prefix: &str, name: &str) -> Option<Subcommand> {
        let name = format!("{}{}", prefix, name);
        let path = Self::resolve_command(prefix, &name);
        path.map(|path| Subcommand { name, path })
    }
}

impl SubcommandsProvider {
    pub fn iter(&self) -> impl Iterator<Item = &Subcommand> {
        self.commands.iter()
    }
}

#[cfg(unix)]
impl SubcommandsProvider {
    fn filter_file(prefix: &str, path: &Path) -> bool {
        use std::os::unix::prelude::*;

        let file_name = path.file_name();
        let Some(entry_name) = file_name.and_then(|name| name.to_str()) else {
            // skip files with invalid names.
            return false;
        };

        if entry_name.starts_with(".") || entry_name.ends_with("~") {
            // skip hidden and backup files.
            return false;
        }

        if !entry_name.starts_with(prefix) {
            // skip non-matching files.
            return false;
        }

        let Ok(metadata) = std::fs::metadata(path) else {
            // couldn't get metadata.
            return false;
        };

        if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
            // skip non-executable files.
            return false;
        }

        true
    }

    fn collect_commands(prefix: &str) -> Vec<PathBuf> {
        let Some(paths) = std::env::var_os("PATH") else {
            // PATH variable is not set.
            return vec![];
        };

        let mut result = vec![];
        for path in std::env::split_paths(&paths) {
            let Ok(dir) = std::fs::read_dir(path) else {
                continue;
            };

            for entry in dir {
                let Ok(entry) = entry else {
                    // invalid entry.
                    continue;
                };

                let path = entry.path();
                if Self::filter_file(prefix, &path) {
                    result.push(path);
                }
            }
        }

        result
    }

    fn resolve_command(prefix: &str, command: &str) -> Option<PathBuf> {
        let Some(paths) = std::env::var_os("PATH") else {
            // PATH variable is not set.
            return None;
        };

        for path in std::env::split_paths(&paths) {
            let path = path.join(command);

            if !path.exists() {
                continue;
            }

            if !Self::filter_file(prefix, &path) {
                continue;
            }

            return Some(path);
        }

        None
    }
}

#[cfg(windows)]
impl SubcommandsProvider {
    fn get_path_exts() -> Option<Vec<String>> {
        let Ok(exts) = std::env::var("PATHEXT") else {
            // PATHEXT variable is not set.
            return None;
        };

        // NOTE: I am not sure if std::env::split_paths should be applied here,
        // since it also deals with '"' which seems to not be used in PATHEXT?
        return Some(
            exts.split(';')
                .map(|ext| ext[1..].to_lowercase())
                .collect::<Vec<_>>(),
        );
    }

    fn filter_file(prefix: &str, path: &Path, exts: Option<&[String]>) -> bool {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x00000002;

        let file_name = path.file_name();
        let Some(entry_name) = file_name.and_then(|name| name.to_str()) else {
            // skip files with invalid names.
            return false;
        };

        if !entry_name.starts_with(prefix) {
            // skip non-matching files.
            return false;
        }

        if let Some(exts) = exts {
            let Some(entry_ext) = path.extension().and_then(|ext| ext.to_str()) else {
                // skip files without extensions.
                return false;
            };

            let entry_ext = entry_ext.to_lowercase();
            if !exts.contains(&entry_ext) {
                // skip non-executable files
                return false;
            }
        }

        let Ok(metadata) = std::fs::metadata(path) else {
            // couldn't get metadata.
            return false;
        };

        let is_hidden = metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0;
        if is_hidden {
            // skip hidden files
            return false;
        }

        true
    }

    fn collect_commands(prefix: &str) -> Vec<PathBuf> {
        let Some(paths) = std::env::var_os("PATH") else {
            // PATH variable is not set.
            return vec![];
        };

        let Some(exts) = Self::get_path_exts() else {
            // PATHEXT variable is not set or invalid.
            return vec![];
        };

        let mut result = vec![];
        for path in std::env::split_paths(&paths) {
            let Ok(dir) = std::fs::read_dir(path) else {
                continue;
            };

            for entry in dir {
                let Ok(entry) = entry else {
                    // invalid entry.
                    continue;
                };

                let path = entry.path();
                if Self::filter_file(prefix, &path, Some(&exts)) {
                    result.push(path);
                }
            }
        }

        result
    }

    fn resolve_command(prefix: &str, command: &str) -> Option<PathBuf> {
        let Some(paths) = std::env::var_os("PATH") else {
            // PATH variable is not set.
            return None;
        };

        let Some(exts) = Self::get_path_exts() else {
            // PATHEXT variable is not set or invalid.
            return None;
        };

        for path in std::env::split_paths(&paths) {
            let mut path = path.join(command);

            // Extension is provided. Just check if file exists.
            if path.extension().is_some() {
                match path.exists() {
                    true if Self::filter_file(prefix, &path, None) => return Some(path),
                    _ => continue,
                }
            }

            // Iterate extensions and check if file exists.
            for ext in &exts {
                path.set_extension(ext);

                match path.exists() {
                    true if Self::filter_file(prefix, &path, None) => return Some(path),
                    _ => continue,
                }
            }
        }

        None
    }
}
