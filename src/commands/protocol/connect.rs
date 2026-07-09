// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError};
use asimov_id::Id;
use asimov_protocol::{CsvHandleResolver, HandleResolver};
use asimov_protocol::{EndpointTicket, Node, Ticket};
use color_print::ceprintln;

pub async fn connect(
    id: &Id,
    _ticket: &Option<String>,
    _flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    let mut resolver = CsvHandleResolver::open("examples/resolve.csv").await?; // TODO

    // Parse the peer ticket provided by the user:
    // let peer_ticket = EndpointTicket::decode_string(ticket.as_ref()).unwrap();

    // Start a node and accept connections from peers:
    let node = Node::default().bind().await?.start().await?;
    node.online().await;

    let Some(endpoint) = resolver.resolve_first(id.clone()).await? else {
        return Err(SysexitsError::EX_NOUSER);
    };

    // Connect directly to the remote peer endpoint:
    let connection = node.connect(endpoint).await?;

    // Print out the peer's hello metadata:
    ceprintln!("<s,g>✓</> {:?}", connection.hello());

    // Close the connection and shut down the node:
    node.terminate().await?;

    Ok(())
}
