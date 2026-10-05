// This is free and unencumbered software released into the public domain.

mod body_logger;
mod proxy_config;
mod proxy_connector;
mod proxy_stream;

use self::{body_logger::BodyLogger, proxy_config::ProxyConfig, proxy_connector::ProxyConnector};
use crate::{BoxError, StandardOptions};
use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Request, State},
    http::{self, HeaderMap, HeaderValue, StatusCode, Version},
    response::Response,
    routing::any,
};
use clientele::crates::clap::Args;
use http_body_util::{BodyExt, Full};
use hyper_rustls::{ConfigBuilderExt as _, HttpsConnector};
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use std::{net::IpAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;

const UPSTREAM_BASE_URL: &str = "https://openrouter.ai/api";
const UPSTREAM_HOST: &str = "openrouter.ai";
const DEFAULT_MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

/// The upstream HTTP client: a hyper client speaking rustls-based TLS to the
/// target, over a connection that is either direct or tunneled through a
/// proxy (see the `connector` module).
type UpstreamClient = Client<HttpsConnector<ProxyConnector>, Full<Bytes>>;

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

#[derive(Clone)]
struct ProxyState {
    client: UpstreamClient,
    logger: Option<BodyLogger>,
    authorization: HeaderValue,
    verbose: bool,
    max_body_bytes: usize,
    upstream_timeout: Duration,
}

pub async fn serve(args: ProxyServeArgs, flags: &StandardOptions) -> Result<(), BoxError> {
    let authorization = authorization_header(std::env::var("OPENROUTER_API_KEY").ok().as_deref())?;
    let addr = super::endpoint::bind_address(args.bind, args.port)?;

    // The TLS configuration, shared between connections to the target and to
    // any `https://` proxy:
    let tls_config = Arc::new(
        rustls::ClientConfig::builder()
            .with_native_roots()?
            .with_no_client_auth(),
    );

    // The upstream proxy (if any), configured through the conventional
    // `https_proxy`/`HTTPS_PROXY`/`all_proxy`/`ALL_PROXY`/`no_proxy`
    // environment variables:
    let proxy_config = ProxyConfig::from_env(UPSTREAM_HOST).map_err(|err| -> BoxError { err })?;
    if flags.verbose > 0 && !matches!(proxy_config, ProxyConfig::Direct) {
        eprintln!("Using upstream proxy: {:?}", proxy_config);
    }

    let proxy_connector = ProxyConnector::new(proxy_config, Arc::clone(&tls_config));
    let https_connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config((*tls_config).clone())
        .https_only()
        .enable_http1()
        .wrap_connector(proxy_connector);
    let client: UpstreamClient = Client::builder(TokioExecutor::new()).build(https_connector);

    let state = ProxyState {
        client,
        logger: BodyLogger::from_env()?, // reads ASIMOV_PROXY_LOG_FILE
        authorization,
        verbose: flags.verbose > 0,
        max_body_bytes: args.max_body_bytes.unwrap_or(DEFAULT_MAX_BODY_BYTES),
        upstream_timeout: Duration::from_secs(args.upstream_timeout.unwrap_or(120)),
    };

    let router = Router::new()
        .route("/{*path}", any(proxy_handler))
        .with_state(state);

    let listener = TcpListener::bind(addr).await.map_err(|error| {
        eprintln!("error: failed to bind proxy listener at {addr}: {error}");
        crate::SysexitsError::EX_UNAVAILABLE
    })?;

    let shutdown = shutdown_signal().map_err(|error| {
        eprintln!("error: failed to install proxy shutdown handlers: {error}");
        crate::SysexitsError::EX_OSERR
    })?;

    if flags.verbose > 0 {
        let addr = listener.local_addr()?;
        eprintln!("Listening on {}...", addr);
    }

    serve_until_shutdown(listener, router, shutdown)
        .await
        .map_err(|error| {
            eprintln!("error: proxy server failed: {error}");
            crate::SysexitsError::EX_IOERR
        })?;
    Ok(())
}

async fn serve_until_shutdown(
    listener: TcpListener,
    router: Router,
    shutdown: impl core::future::Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await
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

async fn proxy_handler(
    State(state): State<ProxyState>,
    req: Request,
) -> Result<Response, StatusCode> {
    let request_path = req.uri().path();
    let request_query = req
        .uri()
        .query()
        .map(|q| format!("?{q}"))
        .unwrap_or_default();

    if state.verbose {
        eprintln!("Proxying request: {} {}", request_path, request_query);
    }

    // https://openrouter.ai/api/v1/chat/completions
    let target_url = format!("{}{}{}", UPSTREAM_BASE_URL, request_path, request_query);

    let (mut head, body) = req.into_parts();

    let body_bytes = read_request_body(body, state.max_body_bytes).await?;

    // Patch the request body before forwarding it upstream:
    let upstream_request_body = patch_request_body(body_bytes)?;

    if let Some(logger) = &state.logger {
        logger.log_request_body(&upstream_request_body);
    }

    // Retarget the request at the upstream server:
    head.uri = target_url
        .parse()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    head.version = Version::HTTP_11; // regardless of the inbound HTTP version

    // Modify request headers:
    strip_hop_by_hop_headers(&mut head.headers);
    head.headers.remove("host"); // don't send "Host: 127.0.0.1"
    head.headers.remove("content-length"); // patching may change the length; hyper recomputes it
    head.headers
        .insert("Authorization", state.authorization.clone());

    // See: https://openrouter.ai/docs/app-attribution
    insert_attribution_headers(&mut head.headers);

    let upstream_request = http::Request::from_parts(head, Full::new(upstream_request_body));

    let upstream_response = wait_for_upstream(
        state.upstream_timeout,
        state.client.request(upstream_request),
    )
    .await?;

    Ok(forward_response(upstream_response, state.logger.clone()))
}

fn forward_response(
    response: http::Response<hyper::body::Incoming>,
    logger: Option<BodyLogger>,
) -> Response {
    // Stream the upstream response body back to the client, teeing each data
    // frame into the body log (if enabled):
    let (mut head, upstream_response_body) = response.into_parts();
    strip_hop_by_hop_headers(&mut head.headers);
    let upstream_response_body = upstream_response_body.map_frame(move |frame| {
        if let (Some(logger), Some(data)) = (&logger, frame.data_ref()) {
            logger.log_response_chunk(data);
        }
        frame
    });

    Response::from_parts(head, Body::new(upstream_response_body))
}

fn strip_hop_by_hop_headers(headers: &mut HeaderMap) {
    let nominated: Vec<_> = headers
        .get_all(http::header::CONNECTION)
        .iter()
        .flat_map(|value| value.as_bytes().split(|byte| *byte == b','))
        .filter_map(|name| http::header::HeaderName::from_bytes(name.trim_ascii()).ok())
        .collect();
    for name in nominated {
        headers.remove(name);
    }
    for name in [
        "connection",
        "keep-alive",
        "proxy-connection",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
    ] {
        headers.remove(name);
    }
}

async fn wait_for_upstream<T, E: core::fmt::Display>(
    limit: Duration,
    request: impl core::future::Future<Output = Result<T, E>>,
) -> Result<T, StatusCode> {
    match tokio::time::timeout(limit, request).await {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(error)) => {
            eprintln!("Upstream request failed: {error}");
            Err(StatusCode::BAD_GATEWAY)
        },
        Err(_) => {
            eprintln!("Upstream response headers timed out");
            Err(StatusCode::GATEWAY_TIMEOUT)
        },
    }
}

async fn read_request_body(body: Body, limit: usize) -> Result<Bytes, StatusCode> {
    use core::error::Error;
    axum::body::to_bytes(body, limit).await.map_err(|error| {
        if error
            .source()
            .is_some_and(|cause| cause.is::<http_body_util::LengthLimitError>())
        {
            StatusCode::PAYLOAD_TOO_LARGE
        } else {
            StatusCode::BAD_REQUEST
        }
    })
}

/// Patches the upstream request body before it is forwarded.
///
/// TODO: Rewrite the `model` property using `jsonc_parser`'s CST API, which
/// preserves the original formatting and whitespace of the request body:
///
/// ```ignore
/// let text = str::from_utf8(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
/// let root = jsonc_parser::cst::CstRootNode::parse(text, &Default::default())
///     .map_err(|_| StatusCode::BAD_REQUEST)?;
/// let object = root.object_value().ok_or(StatusCode::BAD_REQUEST)?;
/// if let Some(model) = object.get("model") { /* rewrite the value */ }
/// Ok(root.to_string().into())
/// ```
fn patch_request_body(body: Bytes) -> Result<Bytes, StatusCode> {
    // For now, the body is forwarded unmodified.
    Ok(body)
}

fn authorization_header(api_key: Option<&str>) -> Result<HeaderValue, crate::SysexitsError> {
    let Some(api_key) = api_key.filter(|key| !key.trim().is_empty()) else {
        eprintln!("error: OPENROUTER_API_KEY must be set and nonempty");
        return Err(crate::SysexitsError::EX_CONFIG);
    };
    let mut header = HeaderValue::from_str(&format!("Bearer {api_key}")).map_err(|_| {
        eprintln!("error: OPENROUTER_API_KEY contains invalid HTTP header bytes");
        crate::SysexitsError::EX_CONFIG
    })?;
    header.set_sensitive(true);
    Ok(header)
}

fn insert_attribution_headers(headers: &mut HeaderMap<HeaderValue>) {
    // See: https://openrouter.ai/docs/app-attribution
    headers.insert(
        "HTTP-Referer",
        HeaderValue::from_static("https://asimov.sh"),
    );
    headers.insert("X-OpenRouter-Title", HeaderValue::from_static("ASIMOV"));
    headers.insert(
        "X-OpenRouter-Categories",
        HeaderValue::from_static("cli-agent,personal-agent"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_standard_and_connection_nominated_headers() {
        let mut headers = HeaderMap::new();
        for name in [
            "keep-alive",
            "proxy-connection",
            "proxy-authenticate",
            "proxy-authorization",
            "te",
            "trailer",
            "transfer-encoding",
            "upgrade",
            "x-first",
            "x-second",
        ] {
            headers.insert(
                http::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_static("value"),
            );
        }
        headers.append(
            "connection",
            HeaderValue::from_static("keep-alive, X-First"),
        );
        headers.append(
            "connection",
            HeaderValue::from_static(" X-Second , invalid name"),
        );
        headers.insert(
            "content-type",
            HeaderValue::from_static("text/event-stream"),
        );
        headers.insert("x-end-to-end", HeaderValue::from_static("keep"));
        strip_hop_by_hop_headers(&mut headers);
        assert_eq!(headers.len(), 2);
        assert_eq!(headers["content-type"], "text/event-stream");
        assert_eq!(headers["x-end-to-end"], "keep");
    }

    #[tokio::test]
    async fn sanitized_chunked_responses_remain_streaming() {
        use tokio::io::AsyncWriteExt;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (release, released) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            crate::shared::test_http::read_request_headers(&mut socket).await;
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive, x-private\r\nX-Private: secret\r\nProxy-Authenticate: Basic\r\nTransfer-Encoding: chunked\r\n\r\nd\r\ndata: first\n\n\r\n").await.unwrap();
            released.await.unwrap();
            socket
                .write_all(b"e\r\ndata: second\n\n\r\n0\r\n\r\n")
                .await
                .unwrap();
        });
        let client = Client::builder(TokioExecutor::new()).build_http::<Full<Bytes>>();
        let request = http::Request::builder()
            .uri(format!("http://{address}/"))
            .body(Full::new(Bytes::new()))
            .unwrap();
        let upstream = tokio::time::timeout(Duration::from_secs(2), client.request(request))
            .await
            .unwrap()
            .unwrap();
        let mut response = forward_response(upstream, None);
        for name in [
            "connection",
            "x-private",
            "proxy-authenticate",
            "transfer-encoding",
        ] {
            assert!(!response.headers().contains_key(name));
        }
        assert_eq!(response.headers()["content-type"], "text/event-stream");
        let first = tokio::time::timeout(Duration::from_secs(2), response.body_mut().frame())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(first.into_data().unwrap(), "data: first\n\n");
        release.send(()).unwrap();
        let rest = tokio::time::timeout(Duration::from_secs(2), response.into_body().collect())
            .await
            .unwrap()
            .unwrap()
            .to_bytes();
        assert_eq!(rest, "data: second\n\n");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn graceful_shutdown_allows_in_flight_responses_to_finish() {
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let router = Router::new().route(
            "/",
            any({
                let entered = entered.clone();
                let release = release.clone();
                move || {
                    let entered = entered.clone();
                    let release = release.clone();
                    async move {
                        entered.notify_one();
                        release.notified().await;
                        "completed response"
                    }
                }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(serve_until_shutdown(listener, router, async {
            let _ = stopped.await;
        }));
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let request = tokio::spawn(async move {
            client
                .get(format!("http://{address}/"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        });
        tokio::time::timeout(Duration::from_secs(2), entered.notified())
            .await
            .unwrap();
        stop.send(()).unwrap();
        tokio::task::yield_now().await;
        assert!(!server.is_finished());
        release.notify_one();
        assert_eq!(request.await.unwrap(), "completed response");
        tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn upstream_deadline_bounds_headers_without_buffering_the_body() {
        use tokio::io::AsyncWriteExt;
        for send_headers in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                crate::shared::test_http::read_request_headers(&mut socket).await;
                if send_headers {
                    socket
                        .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n")
                        .await
                        .unwrap();
                }
                core::future::pending::<()>().await;
            });
            let client = Client::builder(TokioExecutor::new()).build_http::<Full<Bytes>>();
            let request = http::Request::builder()
                .uri(format!("http://{address}/"))
                .body(Full::new(Bytes::new()))
                .unwrap();
            let result = tokio::time::timeout(
                Duration::from_secs(2),
                wait_for_upstream(Duration::from_millis(100), client.request(request)),
            )
            .await;
            server.abort();
            let _ = server.await;
            let result = result.expect("header wait must finish");
            if send_headers {
                assert_eq!(result.unwrap().status(), StatusCode::OK);
            } else {
                assert_eq!(result.unwrap_err(), StatusCode::GATEWAY_TIMEOUT);
            }
        }
    }

    #[tokio::test]
    async fn request_body_limits_apply_to_buffered_and_streamed_bodies() {
        assert_eq!(
            read_request_body(Body::from("1234"), 4).await.unwrap(),
            "1234"
        );
        assert_eq!(
            read_request_body(Body::from("12345"), 4).await.unwrap_err(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        let chunks = [
            Ok::<_, std::io::Error>(Bytes::from_static(b"12")),
            Ok(Bytes::from_static(b"345")),
        ];
        let body = Body::from_stream(futures_lite::stream::iter(chunks));
        assert_eq!(
            read_request_body(body, 4).await.unwrap_err(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert!(
            read_request_body(Body::empty(), 0)
                .await
                .unwrap()
                .is_empty()
        );
        let body = Body::from_stream(futures_lite::stream::iter([Err::<Bytes, _>(
            std::io::Error::other("interrupted upload"),
        )]));
        assert_eq!(
            read_request_body(body, 4).await.unwrap_err(),
            StatusCode::BAD_REQUEST
        );
    }

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
    }

    #[test]
    fn authorization_is_validated_and_marked_sensitive() {
        let header = authorization_header(Some("test-key")).unwrap();
        assert_eq!(header, "Bearer test-key");
        assert!(header.is_sensitive());
        assert!(!format!("{header:?}").contains("test-key"));
        for key in [
            None,
            Some(""),
            Some("   "),
            Some("secret\nvalue"),
            Some("secret\rvalue"),
        ] {
            assert_eq!(
                authorization_header(key).unwrap_err(),
                crate::SysexitsError::EX_CONFIG
            );
        }
    }
}
