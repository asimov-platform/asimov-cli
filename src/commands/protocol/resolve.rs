// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError};
use asimov_directory::fs::{HandleResolver, ResolveHandle};
use asimov_id::Id;
use color_print::ceprintln;
use futures_lite::{pin, stream::StreamExt};

pub async fn resolve(id: &Id, _flags: &StandardOptions) -> Result<(), SysexitsError> {
    let mut resolver = HandleResolver::default().await?;
    let endpoints = resolver.resolve_all(id.clone());
    pin!(endpoints);

    while let Some(endpoint) = endpoints.next().await {
        ceprintln!("{}", endpoint?);
    }

    Ok(())
}
