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
use std::{net::IpAddr, sync::Arc};
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
}

#[derive(Clone)]
struct ProxyState {
    client: UpstreamClient,
    logger: Option<BodyLogger>,
    authorization: HeaderValue,
    verbose: bool,
    max_body_bytes: usize,
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
    };

    let router = Router::new()
        .route("/{*path}", any(proxy_handler))
        .with_state(state);

    let listener = TcpListener::bind(addr).await.map_err(|error| {
        eprintln!("error: failed to bind proxy listener at {addr}: {error}");
        crate::SysexitsError::EX_UNAVAILABLE
    })?;

    if flags.verbose > 0 {
        let addr = listener.local_addr()?;
        eprintln!("Listening on {}...", addr);
    }

    axum::serve(listener, router).await.map_err(|error| {
        eprintln!("error: proxy server failed: {error}");
        crate::SysexitsError::EX_IOERR
    })?;
    Ok(())
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
    head.headers.remove("host"); // don't send "Host: 127.0.0.1"
    head.headers.remove("content-length"); // patching may change the length; hyper recomputes it
    head.headers
        .insert("Authorization", state.authorization.clone());

    // See: https://openrouter.ai/docs/app-attribution
    insert_attribution_headers(&mut head.headers);

    let upstream_request = http::Request::from_parts(head, Full::new(upstream_request_body));

    let upstream_response = state
        .client
        .request(upstream_request)
        .await
        .map_err(|err| {
            eprintln!("Upstream request failed: {}", err);
            StatusCode::BAD_GATEWAY
        })?;

    // Stream the upstream response body back to the client, teeing each data
    // frame into the body log (if enabled):
    let (head, upstream_response_body) = upstream_response.into_parts();
    let logger = state.logger.clone();
    let upstream_response_body = upstream_response_body.map_frame(move |frame| {
        if let (Some(logger), Some(data)) = (&logger, frame.data_ref()) {
            logger.log_response_chunk(data);
        }
        frame
    });

    let response = Response::from_parts(head, Body::new(upstream_response_body));
    Ok(response)
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
