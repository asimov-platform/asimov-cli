// This is free and unencumbered software released into the public domain.

use super::{ModuleMetadata, ModuleType, http::http_client};
use known_types_pypi::PackageMetadata;
use reqwest::Error;

/// Fetches JSON metadata for the current `asimov-modules` package.
pub async fn fetch_current_modules() -> Result<String, Error> {
    fetch_modules("25.0.0.dev0").await // FIXME
}

/// Fetches JSON metadata for a specific `asimov-modules` package version.
pub async fn fetch_modules(version: &str) -> Result<String, Error> {
    let url = format!("https://pypi.org/pypi/asimov-modules/{}/json", version);
    http_client().get(&url).send().await?.text().await
}

/// Parses JSON metadata for the `asimov-modules` package and extracts module
/// names from its runtime dependencies, removing the "asimov-" prefix and
/// "-module" suffix.
pub fn extract_module_names(json_str: impl AsRef<str>) -> serde_json::Result<Vec<ModuleMetadata>> {
    let package: PackageMetadata = serde_json::from_str(json_str.as_ref())?;

    // Extract the dependencies:
    let Some(dependencies) = package.info.requires_dist else {
        return Ok(Vec::new()); // no dependencies found
    };

    // Filter and transform the dependencies:
    let module_names = dependencies
        .into_iter()
        .filter_map(|dep| {
            // Extract the module name part (before any version specifiers):
            let dep_name = dep
                .split(|c| c == ' ' || c == '<' || c == '>' || c == ';' || c == '[')
                .next()?;

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
                    version: package.info.version.clone(),
                    r#type: ModuleType::Python,
                    url: format!("https://pypi.org/project/{}/", dep_name),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(module_names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_module_names() {
        let json = r#"{
            "info": {
                "name": "asimov-modules",
                "version": "25.0.0.dev0",
                "requires_dist": [
                    "asimov-mlx-module",
                    "asimov-gpu-module>=1.0.0",
                    "asimov-cpu-module; python_version >= '3.13'",
                    "numpy>=1.20.0",
                    "other-package"
                ]
            }
        }"#;

        let result: Vec<String> = extract_module_names(json)
            .unwrap()
            .iter()
            .map(|m| m.name.clone())
            .collect();
        assert_eq!(result, vec!["mlx", "gpu", "cpu"]);
    }

    #[test]
    fn test_no_dependencies() {
        let json = r#"{
            "info": {
                "name": "asimov-modules",
                "version": "25.0.0.dev0"
            }
        }"#;

        let result = extract_module_names(json).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_empty_dependencies() {
        let json = r#"{
            "info": {
                "name": "asimov-modules",
                "version": "25.0.0.dev0",
                "requires_dist": []
            }
        }"#;

        let result: Vec<String> = extract_module_names(json)
            .unwrap()
            .iter()
            .map(|m| m.name.clone())
            .collect();
        assert!(result.is_empty());
    }
}
