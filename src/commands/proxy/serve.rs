// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Request, State},
    http::{self, HeaderMap, HeaderValue, StatusCode, Version},
    response::Response,
    routing::any,
};
use core::error::Error;
use http_body_util::{BodyExt, Full};
use hyper_rustls::HttpsConnector;
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use std::net::{IpAddr, SocketAddr};
use tokio::net::TcpListener;

/// The upstream HTTP client: a hyper client over a rustls-based TLS connector.
///
/// TODO: To support HTTPS proxies, SOCKS proxies, and eventually Tor (via the
/// `arti-client` crate), swap the connector stack here for one that tunnels
/// through the configured proxy (e.g. wrapping `HttpConnector` with a
/// CONNECT/SOCKS connector, or replacing it with an Arti-based connector).
type UpstreamClient = Client<HttpsConnector<HttpConnector>, Full<Bytes>>;

pub async fn serve(flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
    let _openrouter_api_key =
        std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY should be set");

    let https_connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()?
        .https_only()
        .enable_http1()
        .build();
    let client: UpstreamClient = Client::builder(TokioExecutor::new()).build(https_connector);

    let router = Router::new()
        .route("/{*path}", any(proxy_handler))
        .with_state(client);

    let host: IpAddr = std::env::var("ASIMOV_PROXY_HOST")
        .ok()
        .and_then(|input| input.parse::<IpAddr>().ok())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));
    let port = std::env::var("ASIMOV_PROXY_PORT")
        .ok()
        .and_then(|input| input.parse::<u16>().ok())
        .unwrap_or(1920);
    let addr = SocketAddr::from((host, port));
    let listener = TcpListener::bind(addr).await.unwrap();

    if flags.verbose > 0 {
        let addr = listener.local_addr()?;
        eprintln!("Listening on {}...", addr);
    }

    axum::serve(listener, router).await.unwrap();
    Ok(())
}

async fn proxy_handler(
    State(client): State<UpstreamClient>,
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
    let target_url = format!("https://openrouter.ai/api{}{}", request_path, request_query);

    let (mut head, body) = req.into_parts();

    let body_bytes = body
        .collect()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .to_bytes();

    let upstream_request_body = body_bytes.clone();

    // TODO: Modify the request body

    // Retarget the request at the upstream server:
    head.uri = target_url
        .parse()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    head.version = Version::HTTP_11; // regardless of the inbound HTTP version

    // Modify request headers:
    head.headers.remove("host"); // don't send "Host: 127.0.0.1"
    //parts.headers.remove("content-length"); // TODO: the original Content-Length is now wrong

    head.headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", openrouter_api_key)).unwrap(),
    );

    // See: https://openrouter.ai/docs/app-attribution
    insert_attribution_headers(&mut head.headers);

    let upstream_request = http::Request::from_parts(head, Full::new(upstream_request_body));

    let upstream_response = client
        .request(upstream_request)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    // Stream the upstream response body back to the client:
    let (head, upstream_response_body) = upstream_response.into_parts();
    let response = Response::from_parts(head, Body::new(upstream_response_body));
    Ok(response)
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
