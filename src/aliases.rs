// This is free and unencumbered software released into the public domain.

//! Command aliases.
//!
//! Aliases are resolved by rewriting the command-line arguments before they
//! are parsed: the first subcommand token is looked up in the alias table
//! and, if found, replaced by its expansion.
//!
//! For now the alias table is hardcoded; in the future it is anticipated to
//! be user-configurable via a config file.

use std::ffi::OsString;

/// The table of command aliases, mapping an alias name to its expansion.
pub static ALIASES: &[(&str, &[&str])] = &[
    ("fetch", &["source", "fetch"]),
    ("install", &["module", "install"]),
    ("list", &["source", "list"]),
    ("resolve", &["module", "resolve"]),
    ("snap", &["source", "snap"]),
    ("uninstall", &["module", "uninstall"]),
    ("upgrade", &["module", "upgrade"]),
];

/// Global options that consume a value in a separate argument
/// (e.g. `--color auto`), which must be skipped over when locating
/// the subcommand token.
const OPTIONS_WITH_VALUES: &[&str] = &["--color"];

/// Resolves command aliases by rewriting `args` in place.
///
/// Locates the first subcommand token (the first argument after the program
/// name that isn't an option or an option's value) and, if it names an alias,
/// splices in the expansion. Any arguments following the alias are preserved.
///
/// The `help` subcommand is treated as transparent, so `asimov help fetch`
/// expands to `asimov help source fetch`.
pub fn resolve(args: &mut Vec<OsString>) {
    let mut i = 1; // skip the program name
    while i < args.len() {
        let Some(arg) = args[i].to_str() else {
            return; // non-UTF-8 argument: leave the command line untouched
        };

        // stop at `--`: everything after it is positional
        if arg == "--" {
            return;
        }

        // skip options (and their values), but treat a lone `-` as positional
        if arg.starts_with('-') && arg.len() > 1 {
            i += if OPTIONS_WITH_VALUES.contains(&arg) {
                2
            } else {
                1
            };
            continue;
        }

        // `help` is transparent: expand the alias it's asking about instead
        if arg == "help" {
            i += 1;
            continue;
        }

        // found the subcommand token: expand it if it's an alias
        if let Some((_, expansion)) = ALIASES.iter().find(|(name, _)| *name == arg) {
            args.splice(i..=i, expansion.iter().map(OsString::from));
        }
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(args: &[&str]) -> Vec<String> {
        let mut args: Vec<OsString> = args.iter().map(OsString::from).collect();
        resolve(&mut args);
        args.into_iter()
            .map(|arg| arg.into_string().unwrap())
            .collect()
    }

    #[test]
    fn expands_fetch() {
        assert_eq!(
            resolved(&["asimov", "fetch", "http://example.org"]),
            ["asimov", "source", "fetch", "http://example.org"]
        );
    }

    #[test]
    fn expands_snap() {
        assert_eq!(resolved(&["asimov", "snap"]), ["asimov", "source", "snap"]);
    }

    #[test]
    fn expands_install() {
        assert_eq!(
            resolved(&["asimov", "install", "serpapi"]),
            ["asimov", "module", "install", "serpapi"]
        );
    }

    #[test]
    fn skips_leading_flags() {
        assert_eq!(
            resolved(&["asimov", "-d", "--color", "auto", "fetch", "url"]),
            ["asimov", "-d", "--color", "auto", "source", "fetch", "url"]
        );
    }

    #[test]
    fn leaves_non_aliases_untouched() {
        assert_eq!(
            resolved(&["asimov", "module", "list"]),
            ["asimov", "module", "list"]
        );
    }

    #[test]
    fn only_expands_the_subcommand_position() {
        assert_eq!(
            resolved(&["asimov", "module", "install", "fetch"]),
            ["asimov", "module", "install", "fetch"]
        );
    }

    #[test]
    fn expands_alias_after_help() {
        assert_eq!(
            resolved(&["asimov", "help", "fetch"]),
            ["asimov", "help", "source", "fetch"]
        );
        assert_eq!(
            resolved(&["asimov", "help", "install"]),
            ["asimov", "help", "module", "install"]
        );
    }

    #[test]
    fn leaves_help_for_non_aliases_untouched() {
        assert_eq!(resolved(&["asimov", "help"]), ["asimov", "help"]);
        assert_eq!(
            resolved(&["asimov", "help", "module"]),
            ["asimov", "help", "module"]
        );
    }

    #[test]
    fn ignores_args_after_double_dash() {
        assert_eq!(
            resolved(&["asimov", "--", "fetch"]),
            ["asimov", "--", "fetch"]
        );
    }
}
