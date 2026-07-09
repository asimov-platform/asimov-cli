// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use core::error::Error;

pub async fn list(_flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
    Ok(()) // TODO
}
