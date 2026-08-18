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

use base64::Engine as _;
use core::{
    error::Error,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use hyper::Uri;
use hyper_util::{
    client::legacy::connect::{Connected, Connection},
    rt::TokioIo,
};
use rustls::pki_types::ServerName;
use std::{io, sync::Arc};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};
use tokio_rustls::TlsConnector;
use tokio_socks::tcp::Socks5Stream;
use tower_service::Service;

pub type BoxError = Box<dyn Error + Send + Sync>;

/// How to reach the upstream server.
#[derive(Clone, Debug)]
pub enum ProxyConfig {
    /// Connect directly to the target.
    Direct,

    /// Tunnel through an HTTP(S) proxy using `CONNECT`.
    HttpConnect {
        host: String,
        port: u16,
        /// Whether to speak TLS to the proxy itself (an `https://` proxy).
        tls: bool,
        /// Pre-encoded `Proxy-Authorization: Basic` credentials.
        basic_auth: Option<String>,
    },

    /// Tunnel through a SOCKS5 proxy.
    Socks5 {
        host: String,
        port: u16,
        auth: Option<(String, String)>,
        /// Whether to resolve target DNS through the proxy (`socks5h://`).
        remote_dns: bool,
    },
    //
    // TODO: a `Tor` variant tunneling through the Tor network by means of
    // the `arti-client` crate (its `DataStream` implements `AsyncRead` +
    // `AsyncWrite`, so it slots into `ProxyStream::new()` directly).
}

impl ProxyConfig {
    /// Determines the proxy configuration for `target_host` from the
    /// conventional environment variables.
    pub fn from_env(target_host: &str) -> Result<Self, BoxError> {
        if no_proxy_matches(target_host) {
            return Ok(Self::Direct);
        }
        match env_var(&["https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY"]) {
            Some(input) => Self::parse(&input),
            None => Ok(Self::Direct),
        }
    }

    /// Parses a proxy URL such as `http://user:pass@host:port` or
    /// `socks5h://host:port`.
    pub fn parse(input: &str) -> Result<Self, BoxError> {
        // A bare `host:port` is conventionally an HTTP proxy:
        let url = if input.contains("://") {
            url::Url::parse(input)
        } else {
            url::Url::parse(&format!("http://{}", input))
        }
        .map_err(|err| format!("invalid proxy URL `{}`: {}", input, err))?;

        let host = url
            .host_str()
            .ok_or_else(|| format!("proxy URL `{}` is missing a host", input))?
            .to_string();

        let username = url.username();
        let password = url.password();

        match url.scheme() {
            "http" | "https" => {
                let tls = url.scheme() == "https";
                let port = url.port().unwrap_or(if tls { 443 } else { 80 });
                let basic_auth = if !username.is_empty() || password.is_some() {
                    let credentials = format!("{}:{}", username, password.unwrap_or_default());
                    Some(base64::engine::general_purpose::STANDARD.encode(credentials))
                } else {
                    None
                };
                Ok(Self::HttpConnect {
                    host,
                    port,
                    tls,
                    basic_auth,
                })
            },

            "socks5" | "socks5h" => {
                let port = url.port().unwrap_or(1080);
                let auth = if !username.is_empty() {
                    Some((
                        username.to_string(),
                        password.unwrap_or_default().to_string(),
                    ))
                } else {
                    None
                };
                Ok(Self::Socks5 {
                    host,
                    port,
                    auth,
                    remote_dns: url.scheme() == "socks5h",
                })
            },

            scheme => Err(format!("unsupported proxy scheme: {}", scheme).into()),
        }
    }
}

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

/// The stream types produced by the different `ProxyConfig` variants, unified
/// behind a trait object.
trait AsyncStream: AsyncRead + AsyncWrite + Send + Unpin {}

impl<T: AsyncRead + AsyncWrite + Send + Unpin> AsyncStream for T {}

/// An established (possibly tunneled) connection to the target.
pub struct ProxyStream {
    io: TokioIo<Box<dyn AsyncStream>>,
}

impl ProxyStream {
    fn new(stream: impl AsyncStream + 'static) -> Self {
        Self {
            io: TokioIo::new(Box::new(stream)),
        }
    }
}

impl Connection for ProxyStream {
    fn connected(&self) -> Connected {
        // Note: even when proxied, the connection is always a tunnel carrying
        // end-to-end TLS, so `Connected::proxy(true)` (which switches hyper to
        // absolute-form request URIs) must *not* be set here.
        Connected::new()
    }
}

impl hyper::rt::Read for ProxyStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_read(cx, buf)
    }
}

impl hyper::rt::Write for ProxyStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.io).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_shutdown(cx)
    }
}

/// Returns the first nonempty environment variable among `names`.
fn env_var(names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
}

/// Checks whether `host` is excluded from proxying by `no_proxy`/`NO_PROXY`.
fn no_proxy_matches(host: &str) -> bool {
    let Some(no_proxy) = env_var(&["no_proxy", "NO_PROXY"]) else {
        return false;
    };
    no_proxy_list_matches(&no_proxy, host)
}

fn no_proxy_list_matches(no_proxy: &str, host: &str) -> bool {
    no_proxy.split(',').any(|entry| {
        let entry = entry.trim().trim_start_matches('.');
        !entry.is_empty()
            && (entry == "*"
                || host.eq_ignore_ascii_case(entry)
                || (host.len() > entry.len()
                    && host[..host.len() - entry.len()].ends_with('.')
                    && host[host.len() - entry.len()..].eq_ignore_ascii_case(entry)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_http_proxy() {
        let config = ProxyConfig::parse("http://proxy.example:3128").unwrap();
        let ProxyConfig::HttpConnect {
            host,
            port,
            tls,
            basic_auth,
        } = config
        else {
            panic!("expected HttpConnect, got: {:?}", config)
        };
        assert_eq!(host, "proxy.example");
        assert_eq!(port, 3128);
        assert!(!tls);
        assert!(basic_auth.is_none());
    }

    #[test]
    fn parse_bare_host_port_as_http_proxy() {
        let config = ProxyConfig::parse("proxy.example:8080").unwrap();
        assert!(matches!(
            config,
            ProxyConfig::HttpConnect {
                tls: false,
                port: 8080,
                ..
            }
        ));
    }

    #[test]
    fn parse_https_proxy_with_auth() {
        let config = ProxyConfig::parse("https://user:pass@proxy.example").unwrap();
        let ProxyConfig::HttpConnect {
            host,
            port,
            tls,
            basic_auth,
        } = config
        else {
            panic!("expected HttpConnect, got: {:?}", config)
        };
        assert_eq!(host, "proxy.example");
        assert_eq!(port, 443);
        assert!(tls);
        assert_eq!(basic_auth.as_deref(), Some("dXNlcjpwYXNz")); // "user:pass"
    }

    #[test]
    fn parse_socks5_proxy() {
        let config = ProxyConfig::parse("socks5://127.0.0.1").unwrap();
        assert!(matches!(
            config,
            ProxyConfig::Socks5 {
                port: 1080,
                remote_dns: false,
                auth: None,
                ..
            }
        ));

        let config = ProxyConfig::parse("socks5h://user:pass@127.0.0.1:9050").unwrap();
        let ProxyConfig::Socks5 {
            port,
            remote_dns,
            auth,
            ..
        } = config
        else {
            panic!("expected Socks5, got: {:?}", config)
        };
        assert_eq!(port, 9050);
        assert!(remote_dns);
        assert_eq!(auth, Some(("user".to_string(), "pass".to_string())));
    }

    #[test]
    fn parse_unsupported_scheme() {
        assert!(ProxyConfig::parse("ftp://proxy.example").is_err());
    }

    #[test]
    fn no_proxy_matching() {
        assert!(no_proxy_list_matches("*", "openrouter.ai"));
        assert!(no_proxy_list_matches("openrouter.ai", "openrouter.ai"));
        assert!(no_proxy_list_matches(".openrouter.ai", "api.openrouter.ai"));
        assert!(no_proxy_list_matches(
            "example.com, openrouter.ai",
            "openrouter.ai"
        ));
        assert!(!no_proxy_list_matches("example.com", "openrouter.ai"));
        assert!(!no_proxy_list_matches("router.ai", "openrouter.ai"));
    }
}
