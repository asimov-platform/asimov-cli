// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions};
use asimov_id::{Handle, PublicKey};

pub async fn remove(
    _handle: &Handle,
    _endpoints: &[PublicKey],
    _flags: &StandardOptions,
) -> Result<(), BoxError> {
    Ok(()) // TODO
}
