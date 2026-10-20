// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::{string::String, vec::Vec};
use clientele::{StandardOptions, SysexitsError::EX_UNAVAILABLE};
use std::io::Write;

/// Untracks the given resource URLs (not yet implemented).
pub async fn untrack(_urls: Vec<String>, _flags: &StandardOptions) -> Result<(), BoxError> {
    let _ = writeln!(
        std::io::stderr().lock(),
        "asimov: resource untrack is not implemented yet"
    );
    Err(EX_UNAVAILABLE.into())
}
