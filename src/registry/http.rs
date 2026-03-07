// This is free and unencumbered software released into the public domain.

use tokio::time::Duration;

pub fn http_client() -> reqwest::Client {
    // TODO: retry support
    reqwest::Client::builder()
        .user_agent("asimov-module-cli")
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client")
}
