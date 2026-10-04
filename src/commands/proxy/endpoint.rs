// This is free and unencumbered software released into the public domain.

use crate::SysexitsError;
use core::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

/// Resolves CLI overrides before environment values and built-in defaults.
pub(super) fn bind_address(
    host: Option<IpAddr>,
    port: Option<u16>,
) -> Result<SocketAddr, SysexitsError> {
    let host = match host {
        Some(host) => host,
        None => environment("ASIMOV_PROXY_BIND", IpAddr::from([127, 0, 0, 1]))?,
    };
    let port = match port {
        Some(port) => port,
        None => environment("ASIMOV_PROXY_PORT", 1920)?,
    };
    Ok(SocketAddr::new(host, port))
}

fn environment<T: FromStr>(name: &str, default: T) -> Result<T, SysexitsError> {
    match std::env::var(name) {
        Ok(value) => value.parse().map_err(|_| {
            eprintln!("error: invalid value for {name}");
            SysexitsError::EX_CONFIG
        }),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(std::env::VarError::NotUnicode(_)) => {
            eprintln!("error: {name} must contain valid UTF-8");
            Err(SysexitsError::EX_CONFIG)
        },
    }
}
