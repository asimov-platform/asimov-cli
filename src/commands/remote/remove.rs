// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::{format, string::String};
use clientele::StandardOptions;

/// Removes a tracked remote.
pub async fn remove(name: String, _flags: &StandardOptions) -> Result<(), BoxError> {
    super::open()?
        .remove(&name)
        .map_err(|e| super::report(&format!("remove remote {name:?}"), e))
}
