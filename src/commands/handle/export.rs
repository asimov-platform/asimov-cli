// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use asimov_directory::fs::HandleResolver;
use color_print::cprintln;
use core::error::Error;
use futures_lite::{pin, stream::StreamExt};

pub async fn export(_flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
    let mut resolver = HandleResolver::default().await?;
    let records = resolver.records();
    pin!(records);

    while let Some(record) = records.next().await {
        let (handle, endpoint) = record?;
        cprintln!("{},{}", handle, endpoint);
    }

    Ok(())
}
