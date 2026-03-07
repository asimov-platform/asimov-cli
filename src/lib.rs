// This is free and unencumbered software released into the public domain.

pub mod commands;
pub mod features;
pub mod options {}
pub mod registry;
pub mod shared;

pub(crate) mod timestamps;

use clientele::{StandardOptions, SysexitsError};

pub type Result<T = SysexitsError, E = SysexitsError> = std::result::Result<T, E>;

/// Sorts links from a module's manifest in the order that we'd like to display
/// them for the command `link` and for choosing the URL to open for the command
/// `browse`.
pub(crate) fn sort_links(module_name: &str, links: &mut [impl AsRef<str>]) {
    use std::cmp::Reverse;

    links.sort_by_cached_key(|link| {
        let Ok(url) = reqwest::Url::parse(link.as_ref()) else {
            // it's not even a valid url? put it last
            return Reverse(0);
        };

        let Some(host) = url.host_str() else {
            // it doesn't have a host, put it last
            return Reverse(0);
        };

        // give highest priority to github links under our org
        let our_module = link.as_ref().contains("github.com/asimov-modules/") as i8;

        let host_score =
            // give priority to github links
            (host.ends_with("github.com") as i8 * 2)
            // then any of the package indices
            + ((host.ends_with("crates.io") ||
                host.ends_with("pypi.org") ||
                host.ends_with("rubygems.org") ||
                host.ends_with("npmjs.com")) as i8);

        let path_score = {
            let path = url.path();
            // give highest priority to links which contain the exact module name
            (path.contains(&format!("asimov-{module_name}-module")) as i8 * 3)
            // next to links which contain `asimov-`
            + (((path.contains("asimov-")
                // and `-module`
                && path.contains("-module")
                // but not `/asimov-modules/`
                && !path.contains("/asimov-modules/")) as i8) * 2)
            // and finally if the path does contain `/asimov-modules/`
            + (path.contains("/asimov-modules/") as i8)
        };

        // add all the scores together, then reverse it because we want the highest scores first (sort is ascending order)
        // (add 1 to differentiate from the invalid/host-less links that we return early for)
        Reverse(our_module * 5 + host_score + path_score + 1)
    });
}
