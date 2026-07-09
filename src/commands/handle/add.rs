// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use asimov_id::{Handle, PublicKey};
use core::error::Error;

pub async fn add(
    _handle: &Handle,
    _endpoints: &[PublicKey],
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    Ok(()) // TODO
}
