// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError};
use asimov_directory::fs::{HandleResolver, ResolveHandle};
use asimov_id::Id;
use asimov_protocol::Node;
use color_print::ceprintln;
use core::error::Error;

pub async fn connect(
    id: &Id,
    _ticket: &Option<String>,
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    // Parse the peer ticket provided by the user:
    // let peer_ticket = EndpointTicket::decode_string(ticket.as_ref()).unwrap();

    // Start a node and accept connections from peers:
    let node = Node::default().bind().await?.start().await?;
    node.online().await;

    let mut resolver = HandleResolver::default().await?;
    let Some(endpoint) = resolver.resolve_first(id.clone()).await? else {
        return Err(SysexitsError::EX_NOUSER.into());
    };

    // Connect directly to the remote peer endpoint:
    let connection = node.connect(endpoint).await?;

    // Print out the peer's hello metadata:
    ceprintln!("<s,g>✓</> {:?}", connection.hello());

    // Close the connection and shut down the node:
    node.terminate().await?;

    Ok(())
}
