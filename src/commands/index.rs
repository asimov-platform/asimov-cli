// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self},
};
use miette::Result;

pub async fn index(
    _input_urls: Vec<String>,
    _module: Option<String>,
    _flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    Ok(()) // TODO
}
