// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError};
use asimov_protocol::{EndpointTicket, Node, Ticket};
use color_print::ceprintln;

pub async fn connect(ticket: &String, _flags: &StandardOptions) -> Result<(), SysexitsError> {
    // Parse the peer ticket provided by the user:
    let peer_ticket = EndpointTicket::decode_string(ticket.as_ref()).unwrap();

    // Start a node and accept connections from peers:
    let node = Node::default().bind().await?.start().await?;
    node.online().await;

    // Connect directly to the remote peer endpoint:
    let connection = node.connect(peer_ticket.endpoint_addr().clone()).await?;

    // Print out the peer's hello metadata:
    ceprintln!("<s,g>✓</> {:?}", connection.hello());

    // Close the connection and shut down the node:
    node.terminate().await?;

    Ok(())
}
