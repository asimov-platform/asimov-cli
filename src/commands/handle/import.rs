// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use core::error::Error;
use std::path::PathBuf;

pub async fn import(_input: &Vec<PathBuf>, _flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
    Ok(()) // TODO
}
