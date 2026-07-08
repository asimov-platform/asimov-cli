// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError};
use asimov_id::{Handle, PublicKey};

pub async fn add(
    _handle: &Handle,
    _endpoints: &[PublicKey],
    _flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    Ok(()) // TODO
}
