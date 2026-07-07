// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError};
use asimov_id::Handle;
use asimov_protocol::{CsvHandleResolver, HandleResolver};
use color_print::ceprintln;
use futures_lite::{pin, stream::StreamExt};

pub async fn resolve(handle: &Handle, _flags: &StandardOptions) -> Result<(), SysexitsError> {
    let mut resolver = CsvHandleResolver::open("examples/resolve.csv").await?; // TODO

    let results = resolver.resolve_handle(handle.clone());
    pin!(results);

    while let Some(result) = results.next().await {
        ceprintln!("{}", result);
    }

    Ok(())
}
