// This is free and unencumbered software released into the public domain.

use std::path::Path;

use crate::Result;
use asimov_env::paths::asimov_root;
use asimov_module::{ModuleManifest, resolve::Resolver};
use clientele::{Subcommand, SubcommandsProvider, SysexitsError::*};
use miette::{IntoDiagnostic, miette};

pub(crate) fn build_resolver(pattern: &str) -> miette::Result<Resolver> {
    let mut resolver = Resolver::new();

    let module_dir_path = asimov_root().join("modules");
    let module_dir = std::fs::read_dir(&module_dir_path)
        .map_err(|e| miette!("Failed to read module manifest directory: {e}"))?
        .filter_map(Result::ok);

    for entry in module_dir {
        let filename = entry.file_name();

        let Some(filename_str) = filename.to_str() else {
            // invalid UTF-8 in filename
            continue;
        };
        let Some(filename) = filename_str.strip_suffix(".yaml") else {
            // no .yaml extension
            continue;
        };

        let manifest = ModuleManifest::read_manifest(filename).map_err(|e| {
            miette!(
                "Invalid module manifest at `{}`: {}",
                entry.path().display(),
                e
            )
        })?;

        if !manifest
            .provides
            .programs
            .iter()
            .any(|program| program.split('-').next_back().is_some_and(|p| p == pattern))
        {
            continue;
        }

        resolver
            .insert_manifest(&manifest)
            .map_err(|e| miette!("{e}"))?;
    }

    Ok(resolver)
}

/// Locates the given subcommand or prints an error.
pub fn locate_subcommand(name: &str) -> Result<Subcommand> {
    match SubcommandsProvider::find("asimov-", name) {
        Some(cmd) => Ok(cmd),
        None => {
            eprintln!("{}: command not found: {}{}", "asimov", "asimov-", name);
            Err(EX_UNAVAILABLE)
        },
    }
}

pub fn normalize_url(url: &str) -> String {
    // test whether it's a normal, valid, URL
    if let Ok(url) = <url::Url>::parse(url) {
        return url.to_string();
    };

    // all the below cases treat the url as a file path.

    // replace a `~/` prefix with the path to the user's home dir.
    let url = url
        .strip_prefix("~/")
        .map(|path| {
            std::env::home_dir()
                .expect("unable to determine home directory")
                .join(path)
        })
        .unwrap_or_else(|| std::path::PathBuf::from(url));

    // `std::path::Path::canonicalize`:
    // > Returns the canonical, absolute form of the path with all
    // > intermediate components normalized and symbolic links resolved.
    //
    // This will only work if the file actually exists.
    if let Ok(path) = std::path::Path::new(&url)
        .canonicalize()
        .map_err(|_| ())
        .and_then(url::Url::from_file_path)
    {
        return path.to_string();
    };

    // `std::path::absolute`:
    // > Makes the path absolute without accessing the filesystem.
    if let Ok(path) = std::path::absolute(&url)
        .map_err(|_| ())
        .and_then(url::Url::from_file_path)
    {
        return path.to_string();
    }

    // TODO: add `std::path::Path::normalize_lexically` once it stabilizes.
    // https://github.com/rust-lang/rust/issues/134694
    //
    // if let Ok(path) = std::path::Path::new(url).normalize_lexically() {
    //     return url::Url::from_file_path(path).unwrap().to_string();
    // }

    // one last try, test whether the `url` crate accepts it as path without changes.
    if let Ok(path) = url::Url::from_file_path(std::path::Path::new(&url)) {
        return path.to_string();
    }

    // otherwise just convert to a file URL without changes and hope for the best :)
    // (we should not really get here but just in case.)
    format!("file://{}", url.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_normalization() {
        let cases = [
            ("https://example.org/", "https://example.org/"),
            ("near://testnet/123456789", "near://testnet/123456789"),
        ];

        for case in cases {
            assert_eq!(normalize_url(case.0), case.1, "input: {:?}", case.0);
        }

        #[cfg(unix)]
        {
            unsafe { std::env::set_var("HOME", "/home/user") };

            let cases = [
                ("~/path/to/file.txt", "file:///home/user/path/to/file.txt"),
                ("/file with spaces.txt", "file:///file%20with%20spaces.txt"),
                ("/file+with+pluses.txt", "file:///file+with+pluses.txt"),
            ];

            for case in cases {
                assert_eq!(normalize_url(case.0), case.1, "input: {:?}", case.0);
            }

            let cur_dir = std::env::current_dir().unwrap().display().to_string();

            let input = "path/to/file.txt";
            let want = "file://".to_string() + &cur_dir + "/path/to/file.txt";
            assert_eq!(
                normalize_url(input),
                want,
                "relative path should be get added after current directory, input: {:?}",
                input
            );

            let input = "../path/./file.txt";
            let want = "file://".to_string() + &cur_dir + "/../path/file.txt";
            assert_eq!(
                normalize_url(input),
                want,
                "relative path should be get added after current directory, input: {:?}",
                input
            );

            let input = "another-type-of-a-string";
            let want = "file://".to_string() + &cur_dir + "/another-type-of-a-string";
            assert_eq!(
                normalize_url(input),
                want,
                "non-path-looking input should be treated as a file in current directory, input: {:?}",
                input
            );

            let input = "hello\\ world!";
            let want = "file://".to_string() + &cur_dir + "/hello%5C%20world!";
            assert_eq!(
                normalize_url(input),
                want,
                "output should be url encoded, input: {:?}",
                input
            );
        }

        #[cfg(windows)]
        {
            let cwd = std::env::current_dir().unwrap();
            let drive = cwd.to_str().unwrap().chars().next().unwrap();
            let cases = [
                (
                    "/file with spaces.txt",
                    format!("file:///{drive}:/file%20with%20spaces.txt"),
                ),
                (
                    "/file+with+pluses.txt",
                    format!("file:///{drive}:/file+with+pluses.txt"),
                ),
            ];

            for case in cases {
                assert_eq!(normalize_url(case.0), case.1, "input: {:?}", case.0);
            }
        }
    }
}
