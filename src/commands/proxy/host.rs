// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions};

pub async fn host(_flags: &StandardOptions) -> Result<(), BoxError> {
    println!("{}", super::endpoint::bind_address(None, None)?.ip());
    Ok(())
}
