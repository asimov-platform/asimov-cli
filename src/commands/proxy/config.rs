// This is free and unencumbered software released into the public domain.

use super::ProxyConfigTarget;
use crate::StandardOptions;
use core::error::Error;

pub async fn config(
    app: &ProxyConfigTarget,
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    use ProxyConfigTarget::*;
    match app {
        #[cfg(feature = "unstable")]
        LangChain => {
            todo!() // TODO
        },

        #[cfg(feature = "unstable")]
        LlamaIndex => {
            todo!() // TODO
        },

        Zed => {},
    };
    Ok(())
}
