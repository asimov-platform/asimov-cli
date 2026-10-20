// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use clientele::{StandardOptions, SysexitsError::EX_UNAVAILABLE};

/// Lists tracked remotes (not yet implemented).
pub async fn list(_flags: &StandardOptions) -> Result<(), BoxError> {
    eprintln!("asimov: remote list is not implemented yet");
    Err(EX_UNAVAILABLE.into())
}
