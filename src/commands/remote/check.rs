// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::string::String;
use clientele::{StandardOptions, SysexitsError::EX_UNAVAILABLE};

/// Pings a tracked remote (not yet implemented).
pub async fn check(_name: String, _flags: &StandardOptions) -> Result<(), BoxError> {
    eprintln!("asimov: remote check is not implemented yet");
    Err(EX_UNAVAILABLE.into())
}
