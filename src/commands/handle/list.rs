// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use asimov_directory::fs::HandleResolver;
use color_print::ceprintln;
use core::error::Error;
use futures_lite::{pin, stream::StreamExt};

pub async fn list(_flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
    let mut resolver = HandleResolver::default().await?;
    let handles = resolver.handles();
    pin!(handles);

    while let Some(handle) = handles.next().await {
        ceprintln!("{}", handle?);
    }

    Ok(())
}
