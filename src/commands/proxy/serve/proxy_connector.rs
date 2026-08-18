// This is free and unencumbered software released into the public domain.

//! Upstream connection establishment, optionally through a proxy.
//!
//! The proxy is configured through the conventional environment variables,
//! consulted in this order: `https_proxy`, `HTTPS_PROXY`, `all_proxy`,
//! `ALL_PROXY`. (Since the upstream endpoint is always HTTPS, `http_proxy`
//! does not apply.) The `no_proxy`/`NO_PROXY` exclusion list is honored.
//!
//! Supported proxy URL schemes:
//!
//! - `http://` — HTTP proxy (tunneled with `CONNECT`)
//! - `https://` — HTTPS proxy (TLS to the proxy itself, then `CONNECT`)
//! - `socks5://` — SOCKS5 proxy (target DNS resolved locally)
//! - `socks5h://` — SOCKS5 proxy (target DNS resolved by the proxy)

use super::{
    proxy_config::{BoxError, ProxyConfig},
    proxy_stream::ProxyStream,
};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use hyper::Uri;
use rustls::pki_types::ServerName;
use std::sync::Arc;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};
use tokio_rustls::TlsConnector;
use tokio_socks::tcp::Socks5Stream;
use tower_service::Service;

/// A connector for the hyper client that establishes TCP connections either
/// directly or through the configured proxy.
///
/// TLS to the *target* is layered on top by `hyper_rustls::HttpsConnector`;
/// this connector only concerns itself with producing a raw byte stream to
/// the target (possibly tunneled, possibly itself TLS-wrapped in the case of
/// an `https://` proxy).
#[derive(Clone)]
pub struct ProxyConnector {
    config: Arc<ProxyConfig>,
    /// TLS used for connecting to `https://` proxies (not the target).
    proxy_tls: TlsConnector,
}

impl ProxyConnector {
    pub fn new(config: ProxyConfig, tls_config: Arc<rustls::ClientConfig>) -> Self {
        Self {
            config: Arc::new(config),
            proxy_tls: TlsConnector::from(tls_config),
        }
    }
}

impl Service<Uri> for ProxyConnector {
    type Response = ProxyStream;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, dst: Uri) -> Self::Future {
        let config = Arc::clone(&self.config);
        let proxy_tls = self.proxy_tls.clone();

        Box::pin(async move {
            let target_host = dst
                .host()
                .ok_or("target URI is missing a host")?
                .to_string();
            let target_port = dst.port_u16().unwrap_or(match dst.scheme_str() {
                Some("https") => 443,
                _ => 80,
            });

            match &*config {
                ProxyConfig::Direct => {
                    let stream = TcpStream::connect((target_host.as_str(), target_port)).await?;
                    Ok(ProxyStream::new(stream))
                },

                ProxyConfig::HttpConnect {
                    host,
                    port,
                    tls,
                    basic_auth,
                } => {
                    let stream = TcpStream::connect((host.as_str(), *port)).await?;
                    if *tls {
                        let server_name = ServerName::try_from(host.clone())?;
                        let stream = proxy_tls.connect(server_name, stream).await?;
                        let stream =
                            http_connect(stream, &target_host, target_port, basic_auth.as_deref())
                                .await?;
                        Ok(ProxyStream::new(stream))
                    } else {
                        let stream =
                            http_connect(stream, &target_host, target_port, basic_auth.as_deref())
                                .await?;
                        Ok(ProxyStream::new(stream))
                    }
                },

                ProxyConfig::Socks5 {
                    host,
                    port,
                    auth,
                    remote_dns,
                } => {
                    let proxy_addr = (host.as_str(), *port);
                    let stream = if *remote_dns {
                        let target = (target_host, target_port);
                        match auth {
                            Some((user, pass)) => {
                                Socks5Stream::connect_with_password(proxy_addr, target, user, pass)
                                    .await?
                            },
                            None => Socks5Stream::connect(proxy_addr, target).await?,
                        }
                    } else {
                        let target = tokio::net::lookup_host((target_host.as_str(), target_port))
                            .await?
                            .next()
                            .ok_or("failed to resolve the target host")?;
                        match auth {
                            Some((user, pass)) => {
                                Socks5Stream::connect_with_password(proxy_addr, target, user, pass)
                                    .await?
                            },
                            None => Socks5Stream::connect(proxy_addr, target).await?,
                        }
                    };
                    Ok(ProxyStream::new(stream))
                },
            }
        })
    }
}

/// Performs an HTTP/1.1 `CONNECT` handshake over `stream`, returning the
/// stream once the tunnel to `host:port` is established.
async fn http_connect<S>(
    mut stream: S,
    host: &str,
    port: u16,
    basic_auth: Option<&str>,
) -> Result<S, BoxError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut request = format!("CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n");
    if let Some(auth) = basic_auth {
        request.push_str(&format!("Proxy-Authorization: Basic {}\r\n", auth));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).await?;

    let mut response = Vec::with_capacity(1024);
    loop {
        let mut chunk = [0u8; 1024];
        let count = stream.read(&mut chunk).await?;
        if count == 0 {
            return Err("the proxy closed the connection during CONNECT".into());
        }
        response.extend_from_slice(&chunk[..count]);
        if response.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if response.len() > 8192 {
            return Err("oversized CONNECT response from the proxy".into());
        }
    }

    // e.g. "HTTP/1.1 200 Connection established"
    let response = String::from_utf8_lossy(&response);
    let status = response.split_whitespace().nth(1).unwrap_or_default();
    if status != "200" {
        return Err(format!(
            "the proxy refused CONNECT: {}",
            response.lines().next().unwrap_or_default()
        )
        .into());
    }

    Ok(stream)
}
