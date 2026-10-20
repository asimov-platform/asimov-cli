// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::string::String;
use clientele::{StandardOptions, SysexitsError::EX_UNAVAILABLE};

/// Removes a tracked remote (not yet implemented).
pub async fn remove(_name: String, _flags: &StandardOptions) -> Result<(), BoxError> {
    eprintln!("asimov: remote remove is not implemented yet");
    Err(EX_UNAVAILABLE.into())
}
