// This is free and unencumbered software released into the public domain.

use super::{ModuleMetadata, ModuleType, http::http_client};
use reqwest::Error;
use serde::{Deserialize, Serialize};

/// Fetches JSON metadata for the current `asimov-modules` crate.
pub async fn fetch_current_modules() -> Result<String, Error> {
    let url = "https://index.crates.io/as/im/asimov-cli"; // FIXME: asimov-modules
    let json_lines = http_client().get(url).send().await?.text().await?;
    let last_line = json_lines
        .lines()
        .filter(|line| !line.trim().is_empty())
        .last()
        .unwrap_or_default();
    Ok(last_line.to_string())
}

/// Parses JSON metadata for the `asimov-modules` crate and extracts module
/// names from its runtime dependencies, removing the "asimov-" prefix and
/// "-module" suffix.
pub fn extract_module_names(json_str: impl AsRef<str>) -> serde_json::Result<Vec<ModuleMetadata>> {
    let crate_version: CrateVersion = serde_json::from_str(json_str.as_ref())?;

    let module_names = crate_version
        .deps
        .iter()
        .filter_map(|dep| {
            let dep_name = &dep.name;

            // Handle the special case of "asimov-module" separately:
            if dep_name == "asimov-module" {
                return None;
            }

            // Check if the dependency name has the expected prefix and suffix:
            if dep_name.starts_with("asimov-") && dep_name.ends_with("-module") {
                // Strip the "asimov-" prefix and "-module" suffix:
                let mod_name = dep_name.strip_prefix("asimov-")?.strip_suffix("-module")?;

                Some(ModuleMetadata {
                    name: mod_name.to_string(),
                    version: crate_version.vers.clone(),
                    r#type: ModuleType::Rust,
                    url: format!("https://crates.io/crates/{}", dep_name),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(module_names)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CrateVersion {
    pub name: String,
    pub vers: String,
    #[serde(default)]
    pub deps: Vec<Dependency>,
    #[serde(default)]
    pub yanked: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Dependency {
    pub name: String,
    pub req: String,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub kind: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_module_names() {
        let json = r#"{
            "name": "asimov-modules",
            "vers": "25.0.0-dev.0",
            "deps": [
                {
                    "name": "asimov-foobar-module",
                    "req": "^25",
                    "features": [],
                    "optional": false,
                    "default_features": true,
                    "target": null,
                    "kind": "normal"
                },
                {
                    "name": "shadow-rs",
                    "req": "^1.0",
                    "features": [
                        "build",
                        "tzdb"
                    ],
                    "optional": false,
                    "default_features": false,
                    "target": null,
                    "kind": "build"
                }
            ],
            "cksum": "",
            "features": {
                "all": [],
                "default": [
                    "all"
                ],
                "unstable": [
                    "all"
                ]
            },
            "yanked": false,
            "rust_version": "1.85"
        }"#;

        let result: Vec<String> = extract_module_names(json)
            .unwrap()
            .iter()
            .map(|m| m.name.clone())
            .collect();
        assert_eq!(result, vec!["foobar"]);
    }
}
