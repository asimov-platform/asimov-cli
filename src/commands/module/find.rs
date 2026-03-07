// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError};

pub async fn find(
    module_name: impl AsRef<str>,
    _flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let module_name = module_name.as_ref();
    let command_name = format!("{module_name}-module");

    match clientele::SubcommandsProvider::find("asimov-", &command_name) {
        Some(command) => {
            println!("{}", command.path.display());
            Ok(())
        },
        None => {
            eprintln!("unknown module: `{module_name}`");
            Err(SysexitsError::EX_UNAVAILABLE)
        },
    }
}
