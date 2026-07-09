// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError};
use asimov_directory::fs::{HandleResolver, ResolveHandle};
use asimov_id::Id;
use asimov_protocol::Node;
use color_print::ceprintln;

pub async fn ping(
    id: &Id,
    _ticket: &Option<String>,
    _flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    // Parse the peer ticket provided by the user:
    // let peer_ticket = EndpointTicket::decode_string(ticket.as_ref()).unwrap();

    // Start a node and accept connections from peers:
    let node = Node::default().bind().await?.start().await?;
    node.online().await;

    let mut resolver = HandleResolver::default().await?;
    let Some(endpoint) = resolver.resolve_first(id.clone()).await? else {
        return Err(SysexitsError::EX_NOUSER);
    };

    // Connect directly to the remote peer endpoint:
    let rtt = node.ping(endpoint).await?;

    // Print out the round-trip time (RTT):
    ceprintln!("<s,g>✓</> Pinged the peer in <s>{rtt:?}</> RTT");

    // Close the connection and shut down the node:
    node.terminate().await?;

    Ok(())
}
