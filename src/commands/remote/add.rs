// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::{format, string::String};
use clientele::StandardOptions;

/// Adds a tracked HTTP(S) remote without replacing an existing name.
pub async fn add(name: String, url: String, _flags: &StandardOptions) -> Result<(), BoxError> {
    super::open()?
        .add(&name, &url)
        .map_err(|e| super::report(&format!("add remote {name:?}"), e))
}
