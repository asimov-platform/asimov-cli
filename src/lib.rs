// This is free and unencumbered software released into the public domain.

extern crate alloc;

pub mod aliases;
pub mod commands;
pub mod options {}
pub mod registry;
pub mod shared;

#[cfg(all(feature = "source", feature = "source-snap"))]
pub(crate) mod timestamps;

#[cfg(any(
    feature = "module",
    feature = "proxy",
    feature = "source",
    feature = "unstable"
))]
use clientele::StandardOptions;
use clientele::SysexitsError;

pub type BoxError = Box<dyn core::error::Error + Send + Sync>;

pub type Result<T = SysexitsError, E = SysexitsError> = std::result::Result<T, E>;

/// Sorts links from a module's manifest in the order that we'd like to display
/// them for the command `link` and for choosing the URL to open for the command
/// `browse`.
#[cfg(any(feature = "module", test))]
pub(crate) fn sort_links(module_name: &str, links: &mut [impl AsRef<str>]) {
    use core::cmp::Reverse;

    links.sort_by_cached_key(|link| {
        let Ok(url) = reqwest::Url::parse(link.as_ref()) else {
            // it's not even a valid url? put it last
            return Reverse(0);
        };

        let Some(host) = url.host_str() else {
            // it doesn't have a host, put it last
            return Reverse(0);
        };

        let segments: Vec<_> = url.path_segments().into_iter().flatten().collect();
        let is_domain = |domain: &str| {
            host == domain
                || host
                    .strip_suffix(domain)
                    .is_some_and(|prefix| prefix.ends_with('.'))
        };

        // Give highest priority to repositories under our GitHub organization.
        let our_module =
            (host == "github.com" && segments.first() == Some(&"asimov-modules")) as i8;

        let host_score =
            // give priority to github links
            (is_domain("github.com") as i8 * 2)
            // then any of the package indices
            + ((is_domain("crates.io") ||
                is_domain("pypi.org") ||
                is_domain("rubygems.org") ||
                is_domain("npmjs.com")) as i8);

        let path_score = {
            let package = format!("asimov-{module_name}-module");
            // Match complete path segments, never query strings or substrings.
            (segments.contains(&package.as_str()) as i8 * 3)
                + (segments
                    .iter()
                    .any(|segment| segment.starts_with("asimov-") && segment.ends_with("-module"))
                    as i8
                    * 2)
                + (segments.contains(&"asimov-modules") as i8)
        };

        // add all the scores together, then reverse it because we want the highest scores first (sort is ascending order)
        // (add 1 to differentiate from the invalid/host-less links that we return early for)
        Reverse(our_module * 5 + host_score + path_score + 1)
    });
}

#[cfg(test)]
mod tests {
    use super::sort_links;

    #[test]
    fn link_ranking_uses_host_and_path_boundaries() {
        for (preferred, lookalike) in [
            (
                "https://github.com/other/repo",
                "https://evilgithub.com/other/repo",
            ),
            (
                "https://crates.io/crates/demo",
                "https://notcrates.io/crates/demo",
            ),
            (
                "https://pypi.org/project/demo",
                "https://notpypi.org/project/demo",
            ),
            (
                "https://rubygems.org/gems/demo",
                "https://notrubygems.org/gems/demo",
            ),
            (
                "https://www.npmjs.com/package/demo",
                "https://notnpmjs.com/package/demo",
            ),
            (
                "https://github.com/other/repo",
                "https://example.com/?next=https://github.com/asimov-modules/demo",
            ),
            (
                "https://github.com/asimov-modules/demo",
                "https://example.com/github.com/asimov-modules/demo",
            ),
            (
                "https://example.com/asimov-demo-module",
                "https://example.com/not-asimov-demo-module-suffix",
            ),
            (
                "https://example.com/asimov-demo-module",
                "https://example.com/?name=asimov-demo-module",
            ),
            (
                "https://example.com/asimov-other-module",
                "https://example.com/asimov-other/unrelated-module",
            ),
            ("https://example.com/", "not a URL"),
            ("https://example.com/", "mailto:person@example.com"),
        ] {
            let mut links = [lookalike, preferred];
            sort_links("demo", &mut links);
            assert_eq!(links[0], preferred, "{lookalike}");
        }
    }
}
