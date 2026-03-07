// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError};

pub async fn inspect(
    _module_name: impl AsRef<str>,
    _flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    Ok(()) // TODO
}
