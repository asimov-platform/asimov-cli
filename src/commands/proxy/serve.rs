// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, SysexitsError};
use asimov_proxy::{
    BodyLogger, Error, ProxyConfig,
    openai::{Proxy, ProxyOptions},
};
use clientele::crates::clap::Args;
use core::{net::IpAddr, time::Duration};
use tokio::net::TcpListener;

#[derive(Args, Clone, Debug, Default)]
pub struct ProxyServeArgs {
    /// The address to bind to [default: $ASIMOV_PROXY_BIND or 127.0.0.1]
    #[clap(long)]
    pub bind: Option<IpAddr>,

    /// The port to bind to [default: $ASIMOV_PROXY_PORT or 1920]
    #[clap(long)]
    pub port: Option<u16>,

    /// Maximum buffered request body size in bytes [default: 16777216]
    #[clap(long)]
    pub max_body_bytes: Option<usize>,

    /// Seconds to wait for upstream response headers [default: 120]
    #[clap(long, value_parser = clap::value_parser!(u64).range(1..))]
    pub upstream_timeout: Option<u64>,
}

pub async fn serve(args: ProxyServeArgs, flags: &StandardOptions) -> Result<(), BoxError> {
    let api_key = std::env::var("OPENROUTER_API_KEY").unwrap_or_default();
    let addr = super::endpoint::bind_address(args.bind, args.port)?;
    let mut options = ProxyOptions {
        upstream_proxy: ProxyConfig::from_env("openrouter.ai")?,
        logger: match std::env::var("ASIMOV_PROXY_LOG_FILE") {
            Ok(path) if !path.is_empty() => Some(BodyLogger::open(path.as_ref())?),
            _ => None,
        },
        ..ProxyOptions::default()
    };
    if let Some(limit) = args.max_body_bytes {
        options.max_body_bytes = limit;
    }
    if let Some(seconds) = args.upstream_timeout {
        options.upstream_timeout = Duration::from_secs(seconds);
    }
    if flags.verbose > 0 && !matches!(options.upstream_proxy, ProxyConfig::Direct) {
        eprintln!("Using upstream proxy: {:?}", options.upstream_proxy);
    }
    let proxy = Proxy::openrouter(&api_key, options).map_err(|error| -> BoxError {
        match error {
            Error::MissingApiKey | Error::InvalidApiKey => {
                eprintln!("error: OPENROUTER_API_KEY: {error}");
                SysexitsError::EX_CONFIG.into()
            },
            error => error.into(),
        }
    })?;

    let listener = TcpListener::bind(addr).await.map_err(|error| {
        eprintln!("error: failed to bind proxy listener at {addr}: {error}");
        SysexitsError::EX_UNAVAILABLE
    })?;
    let shutdown = shutdown_signal().map_err(|error| {
        eprintln!("error: failed to install proxy shutdown handlers: {error}");
        SysexitsError::EX_OSERR
    })?;
    if flags.verbose > 0 {
        eprintln!("Listening on {}...", listener.local_addr()?);
    }
    proxy.serve(listener, shutdown).await.map_err(|error| {
        eprintln!("error: proxy server failed: {error}");
        SysexitsError::EX_IOERR
    })?;
    Ok(())
}

fn shutdown_signal() -> std::io::Result<impl core::future::Future<Output = ()>> {
    #[cfg(unix)]
    let (mut interrupt, mut terminate) = {
        use tokio::signal::unix::{SignalKind, signal};
        (
            signal(SignalKind::interrupt())?,
            signal(SignalKind::terminate())?,
        )
    };
    Ok(async move {
        #[cfg(unix)]
        tokio::select! {
            _ = interrupt.recv() => {},
            _ = terminate.recv() => {},
        }
        #[cfg(not(unix))]
        if let Err(error) = tokio::signal::ctrl_c().await {
            eprintln!("error: failed to wait for proxy shutdown: {error}");
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_body_limit_is_configurable() {
        #[derive(clap::Parser)]
        struct Args {
            #[command(flatten)]
            args: ProxyServeArgs,
        }
        use clap::Parser;
        assert_eq!(
            Args::try_parse_from(["test", "--max-body-bytes", "1024"])
                .unwrap()
                .args
                .max_body_bytes,
            Some(1024)
        );
        assert!(Args::try_parse_from(["test", "--max-body-bytes", "invalid"]).is_err());
        assert!(Args::try_parse_from(["test", "--upstream-timeout", "0"]).is_err());
    }
}
