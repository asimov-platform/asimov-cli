// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use alloc::{format, string::String};
use asimov_keyring::Keyring;
use asimov_remote::http;
use clientele::{
    StandardOptions,
    SysexitsError::{EX_DATAERR, EX_UNAVAILABLE},
};
use std::io::Write;

/// Checks a remote with a signed OPTIONS request and a verified response.
pub async fn check(name: String, flags: &StandardOptions) -> Result<(), BoxError> {
    let remote = super::open()?
        .get(&name)
        .await
        .map_err(|error| super::report(&format!("read remote {name:?}"), error))?;
    let user = whoami::username().unwrap_or_else(|_| "default".into());
    // Repair the public-key cache before selecting the default bearer identity.
    let key = Keyring::open()
        .and_then(|mut keyring| keyring.ensure_secret_key(&user))
        .map_err(|error| {
            let _ = writeln!(
                std::io::stderr().lock(),
                "asimov: failed to load local signing key for remote {name:?}: {error}"
            );
            EX_UNAVAILABLE
        })?;
    let executor = http::Executor::new(&remote.url, None)
        .map_err(|error| report(&name, error))?
        .with_signing_key(key);
    let rtt = executor
        .check()
        .await
        .map_err(|error| report(&name, error))?;
    if flags.verbose > 0 {
        writeln!(
            std::io::stderr().lock(),
            "asimov: remote {name:?} OK (verified signed response; RTT: {:.3} ms)",
            rtt.as_secs_f64() * 1000.0,
        )?;
    }
    Ok(())
}

fn report(name: &str, error: http::Error) -> BoxError {
    let _ = writeln!(
        std::io::stderr().lock(),
        "asimov: failed to check remote {name:?}: {error}"
    );
    match error {
        http::Error::Integrity(_) => EX_DATAERR,
        _ => EX_UNAVAILABLE,
    }
    .into()
}
