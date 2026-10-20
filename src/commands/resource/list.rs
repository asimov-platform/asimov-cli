// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use clientele::{StandardOptions, SysexitsError::EX_UNAVAILABLE};
use std::io::Write;

mod urls;
pub use urls::ResourceListArgs;

/// Lists tracked resources, or uses module/relay listers for the given URLs.
///
/// Tracked-resource listing without URLs is not yet implemented.
pub async fn list(args: ResourceListArgs, flags: &StandardOptions) -> Result<(), BoxError> {
    if !args.urls.is_empty() {
        return urls::list(args, flags).await;
    }

    let _ = writeln!(
        std::io::stderr().lock(),
        "asimov: listing tracked resources is not implemented yet"
    );
    Err(EX_UNAVAILABLE.into())
}
