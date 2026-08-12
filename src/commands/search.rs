// This is free and unencumbered software released into the public domain.

use crate::{
    StandardOptions,
    SysexitsError::{self},
};
use asimov_module::ModuleName;
use miette::Result;

pub async fn search(
    _prompt: String,
    _module: Option<ModuleName>,
    _flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    Ok(()) // TODO
}
