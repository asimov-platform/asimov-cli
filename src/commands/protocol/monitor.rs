// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError};
use asimov_protocol::{DefaultPreset, Endpoint, EndpointTicket, PING_ALPN, PingProtocol, Router};

pub async fn monitor(_flags: &StandardOptions) -> Result<(), SysexitsError> {
    // Create an endpoint and accept connections from peers:
    let self_endpoint = Endpoint::bind(DefaultPreset).await.unwrap();
    self_endpoint.online().await;

    // Start the ping protocol service router:
    let service = PingProtocol::new();
    let _router = Router::builder(self_endpoint.clone())
        .accept(PING_ALPN, service)
        .spawn();

    // Print out the endpoint's ticket that allows connecting to it:
    let self_ticket = EndpointTicket::new(self_endpoint.addr());
    std::println!("{}", self_ticket);

    // Wait until the user presses Ctrl-C to terminate the program:
    tokio::signal::ctrl_c().await?;

    Ok(())
}
