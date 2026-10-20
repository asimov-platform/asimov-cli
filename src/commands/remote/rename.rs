// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::{format, string::String};
use clientele::StandardOptions;

/// Renames a tracked remote without replacing an existing destination.
pub async fn rename(old: String, new: String, _flags: &StandardOptions) -> Result<(), BoxError> {
    super::open()?
        .rename(&old, &new)
        .map_err(|e| super::report(&format!("rename remote {old:?} to {new:?}"), e))
}
