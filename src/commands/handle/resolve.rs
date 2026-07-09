// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use asimov_directory::fs::{HandleResolver, ResolveHandle};
use asimov_id::{Id, PublicKeyEncoding};
use color_print::ceprintln;
use core::error::Error;
use futures_lite::{pin, stream::StreamExt};

pub async fn resolve(
    id: &Id,
    format: &Option<PublicKeyEncoding>,
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    let format = format.unwrap_or_default();

    let mut resolver = HandleResolver::default().await?;
    let endpoints = resolver.resolve_all(id.clone());
    pin!(endpoints);

    while let Some(endpoint) = endpoints.next().await {
        match endpoint?.encode(format) {
            Some(encoded) => ceprintln!("{}", encoded),
            None => continue,
        }
    }

    Ok(())
}
