// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::{string::String, vec::Vec};
use clientele::{StandardOptions, SysexitsError::EX_UNAVAILABLE};
use std::io::Write;

/// Searches all tracked resources (not yet implemented).
pub async fn search(_query: Vec<String>, _flags: &StandardOptions) -> Result<(), BoxError> {
    let _ = writeln!(
        std::io::stderr().lock(),
        "asimov: resource search is not implemented yet"
    );
    Err(EX_UNAVAILABLE.into())
}
