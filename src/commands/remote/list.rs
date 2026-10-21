// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use clientele::StandardOptions;
use std::io::Write;

/// Lists remote names in sorted order, including URLs with `--verbose`.
pub async fn list(flags: &StandardOptions) -> Result<(), BoxError> {
    let remotes = super::open()?
        .list()
        .await
        .map_err(|e| super::report("list remotes", e))?;
    let mut stdout = std::io::stdout().lock();
    for remote in remotes {
        if flags.verbose > 0 {
            writeln!(stdout, "{}\t{}", remote.name, remote.url)?;
        } else {
            writeln!(stdout, "{}", remote.name)?;
        }
    }
    Ok(())
}
