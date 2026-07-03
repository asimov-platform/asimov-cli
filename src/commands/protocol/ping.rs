// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError};
use asimov_protocol::{DefaultPreset, Endpoint, EndpointTicket, PingProtocol, Ticket};

pub async fn ping(ticket: impl AsRef<str>, _flags: &StandardOptions) -> Result<(), SysexitsError> {
    // Parse the peer ticket provided by the user:
    let peer_ticket = EndpointTicket::decode_string(ticket.as_ref()).unwrap();

    // Connect to the remote peer endpoint:
    let self_endpoint = Endpoint::bind(DefaultPreset).await.unwrap();
    let service = PingProtocol::new();
    let rtt = service
        .ping(&self_endpoint, peer_ticket.endpoint_addr().clone())
        .await?;

    std::eprintln!("Pinged the peer in {rtt:?} RTT");

    self_endpoint.close().await;

    Ok(())
}
