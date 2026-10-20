// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use clientele::{StandardOptions, SysexitsError::EX_UNAVAILABLE};
use std::io::Write;

/// Lists the set of tracked resources (not yet implemented).
pub async fn list(_flags: &StandardOptions) -> Result<(), BoxError> {
    let _ = writeln!(
        std::io::stderr().lock(),
        "asimov: resource list is not implemented yet"
    );
    Err(EX_UNAVAILABLE.into())
}
