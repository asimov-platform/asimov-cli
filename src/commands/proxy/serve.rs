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
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::net::TcpListener;

const UPSTREAM_BASE_URL: &str = "https://openrouter.ai/api";
const UPSTREAM_HOST: &str = "openrouter.ai";

/// The upstream HTTP client: a hyper client speaking rustls-based TLS to the
/// target, over a connection that is either direct or tunneled through a
/// proxy (see the `connector` module).
type UpstreamClient = Client<HttpsConnector<ProxyConnector>, Full<Bytes>>;

#[derive(Args, Clone, Debug, Default)]
pub struct ProxyServeOptions {
    /// The address to bind to [default: $ASIMOV_PROXY_BIND or 127.0.0.1]
    #[clap(long)]
    pub bind: Option<IpAddr>,

    /// The port to bind to [default: $ASIMOV_PROXY_PORT or 1920]
    #[clap(long)]
    pub port: Option<u16>,
}

#[derive(Clone)]
struct ProxyState {
    client: UpstreamClient,
    logger: Option<BodyLogger>,
}

pub async fn serve(options: &ProxyServeOptions, flags: &StandardOptions) -> Result<(), BoxError> {
    let _openrouter_api_key =
        std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY should be set");

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
    };

    let router = Router::new()
        .route("/{*path}", any(proxy_handler))
        .with_state(state);

    let bind: IpAddr = options.bind.unwrap_or_else(|| {
        std::env::var("ASIMOV_PROXY_BIND")
            .ok()
            .and_then(|input| input.parse::<IpAddr>().ok())
            .unwrap_or(IpAddr::from([127, 0, 0, 1]))
    });
    let port = options.port.unwrap_or_else(|| {
        std::env::var("ASIMOV_PROXY_PORT")
            .ok()
            .and_then(|input| input.parse::<u16>().ok())
            .unwrap_or(1920)
    });
    let addr = SocketAddr::from((bind, port));
    let listener = TcpListener::bind(addr).await.unwrap();

    if flags.verbose > 0 {
        let addr = listener.local_addr()?;
        eprintln!("Listening on {}...", addr);
    }

    axum::serve(listener, router).await.unwrap();
    Ok(())
}

async fn proxy_handler(
    State(state): State<ProxyState>,
    req: Request,
) -> Result<Response, StatusCode> {
    let openrouter_api_key =
        std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY should be set");

    let request_path = req.uri().path();
    let request_query = req
        .uri()
        .query()
        .map(|q| format!("?{q}"))
        .unwrap_or_default();

    if true {
        // TODO: flags.verbose > 0
        eprintln!("Proxying request: {} {}", request_path, request_query);
    }

    // https://openrouter.ai/api/v1/chat/completions
    let target_url = format!("{}{}{}", UPSTREAM_BASE_URL, request_path, request_query);

    let (mut head, body) = req.into_parts();

    let body_bytes = body
        .collect()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .to_bytes();

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
    head.headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", openrouter_api_key)).unwrap(),
    );

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
