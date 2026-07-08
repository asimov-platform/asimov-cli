// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError};
use std::path::PathBuf;

pub async fn import(_input: &Vec<PathBuf>, _flags: &StandardOptions) -> Result<(), SysexitsError> {
    Ok(()) // TODO
}
