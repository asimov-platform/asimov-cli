// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self},
};
use miette::Result;

pub async fn search(
    _prompt: &str,
    _module: Option<&str>,
    _flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    Ok(()) // TODO
}
