// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::string::String;
use clientele::{StandardOptions, SysexitsError::EX_UNAVAILABLE};

/// Adds a tracked remote (not yet implemented).
pub async fn add(_name: String, _url: String, _flags: &StandardOptions) -> Result<(), BoxError> {
    eprintln!("asimov: remote add is not implemented yet");
    Err(EX_UNAVAILABLE.into())
}
