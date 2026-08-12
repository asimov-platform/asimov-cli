// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, StatusCode},
    response::Response,
    routing::any,
};
use core::error::Error;
use http_body_util::BodyExt;
use reqwest::Client;
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub async fn serve(_flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let router = Router::new()
        .route("/{*path}", any(proxy_handler))
        .with_state(client);
    let addr = SocketAddr::from(([127, 0, 0, 1], 1920)); // TODO: ASIMOV_PROXY_{HOST,PORT}
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, router).await.unwrap();
    Ok(()) // TODO
}

async fn proxy_handler(State(client): State<Client>, req: Request) -> Result<Response, StatusCode> {
    let request_path = req.uri().path();
    let request_query = req
        .uri()
        .query()
        .map(|q| format!("?{q}"))
        .unwrap_or_default();

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

    // Modify request headers:
    head.headers.remove("host"); // don't send "Host: 127.0.0.1"
    //parts.headers.remove("content-length"); // TODO: the original Content-Length is now wrong

    // # See: https://openrouter.ai/docs/app-attribution
    head.headers.insert(
        "HTTP-Referer",
        HeaderValue::from_static("https://asimov.sh"),
    );
    head.headers
        .insert("X-OpenRouter-Title", HeaderValue::from_static("ASIMOV"));
    head.headers.insert(
        "X-OpenRouter-Categories",
        HeaderValue::from_static("cli-agent,personal-agent"),
    );

    let upstream_request = client
        .request(head.method, &target_url)
        .headers(head.headers)
        .body(upstream_request_body)
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let upstream_response = client
        .execute(upstream_request)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let mut response_builder = Response::builder().status(upstream_response.status());
    for (name, value) in upstream_response.headers() {
        response_builder = response_builder.header(name, value);
    }
    let upstream_response_stream = upstream_response.bytes_stream();
    let upstream_response_body = Body::from_stream(upstream_response_stream);

    let response = response_builder.body(upstream_response_body).unwrap();
    Ok(response)
}
