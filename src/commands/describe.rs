// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self},
};

pub async fn describe(
    _input_urls: Vec<String>,
    _module: Option<String>,
    _output: Option<String>,
    _flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    Ok(()) // TODO
}
