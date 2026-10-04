// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions};

pub async fn url(_flags: &StandardOptions) -> Result<(), BoxError> {
    println!("http://{}/v1", super::endpoint::bind_address(None, None)?);
    Ok(())
}
