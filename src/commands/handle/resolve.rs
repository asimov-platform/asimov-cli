// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError};
use asimov_id::{Id, PublicKeyEncoding};
use asimov_protocol::{CsvHandleResolver, HandleResolver};
use color_print::ceprintln;
use futures_lite::{pin, stream::StreamExt};

pub async fn resolve(
    id: &Id,
    format: &Option<PublicKeyEncoding>,
    _flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let format = format.unwrap_or_default();
    let mut resolver = CsvHandleResolver::open("examples/resolve.csv").await?; // TODO

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
