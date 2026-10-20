// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::string::String;
use clientele::{StandardOptions, SysexitsError::EX_UNAVAILABLE};

/// Renames a tracked remote (not yet implemented).
pub async fn rename(_old: String, _new: String, _flags: &StandardOptions) -> Result<(), BoxError> {
    eprintln!("asimov: remote rename is not implemented yet");
    Err(EX_UNAVAILABLE.into())
}
