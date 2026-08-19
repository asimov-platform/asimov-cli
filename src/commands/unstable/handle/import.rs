// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions};
use std::path::PathBuf;

pub async fn import(_input: &Vec<PathBuf>, _flags: &StandardOptions) -> Result<(), BoxError> {
    Ok(()) // TODO
}
