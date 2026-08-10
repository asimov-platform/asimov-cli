// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self},
};
use asimov_module::ModuleName;

pub async fn describe(
    _input_urls: Vec<String>,
    _module: Option<ModuleName>,
    _output: Option<String>,
    _flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    Ok(()) // TODO
}
