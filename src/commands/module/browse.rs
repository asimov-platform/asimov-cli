// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, SysexitsError::*};
use asimov_module::ModuleName;

pub async fn browse(module_name: ModuleName, _flags: &StandardOptions) -> Result<(), BoxError> {
    let registry = asimov_registry::Registry::default();

    let manifest = registry
        .read_manifest(&module_name)
        .await
        .map_err(|e| {
            tracing::error!("failed to read module manifest: {e}");
            EX_UNAVAILABLE
        })?
        .manifest;

    if let Some(link) = preferred_link(&manifest.name, manifest.links) {
        open::that(&link).inspect_err(|e| tracing::error!("failed to open URL `{link}`: {e}"))?;
        return Ok(());
    }

    eprintln!("unable to browse module: {module_name}");
    Err(EX_UNAVAILABLE.into())
}

fn preferred_link(module_name: &str, mut links: Vec<String>) -> Option<String> {
    links.retain(|link| {
        url::Url::parse(link)
            .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
    });
    crate::sort_links(module_name, &mut links);
    links.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unusable_links_never_become_browser_targets() {
        let invalid: Vec<String> = [
            "broken",
            "file:///tmp/config",
            "mailto:a@example.com",
            "javascript:alert(1)",
            "ftp://github.com/asimov-modules/demo",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert!(preferred_link("demo", Vec::new()).is_none());
        assert!(preferred_link("demo", invalid.clone()).is_none());
        let mut mixed = invalid;
        mixed.push("https://example.com/demo".into());
        mixed.push("https://github.com/asimov-modules/asimov-demo-module".into());
        assert_eq!(
            preferred_link("demo", mixed).as_deref(),
            Some("https://github.com/asimov-modules/asimov-demo-module")
        );
    }
}
