// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, SysexitsError};
use asimov_module::ModuleName;

pub async fn find(module_name: &ModuleName, _flags: &StandardOptions) -> Result<(), BoxError> {
    let command_name = format!("{module_name}-module");

    match clientele::SubcommandsProvider::find("asimov-", &command_name) {
        Some(command) => {
            println!("{}", command.path.display());
            Ok(())
        },
        None => {
            eprintln!("unknown module: `{module_name}`");
            Err(SysexitsError::EX_UNAVAILABLE.into())
        },
    }
}
