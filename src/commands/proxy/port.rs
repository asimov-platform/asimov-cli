// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, shared::outln};

pub async fn port(_flags: &StandardOptions) -> Result<(), BoxError> {
    outln!("{}", super::endpoint::bind_address(None, None)?.port())?;
    Ok(())
}
