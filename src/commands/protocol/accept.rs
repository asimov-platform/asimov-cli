// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions};
use asimov_protocol::{EndpointTicket, Node};
use color_print::ceprintln;

pub async fn accept(_flags: &StandardOptions) -> Result<(), BoxError> {
    // Start a node and accept connections from peers:
    let node = Node::default().bind().await?.start().await?;
    node.online().await;

    // Print out our endpoint's ticket to allow connecting to it:
    let endpoint = node.public_key();
    ceprintln!("{}", endpoint);
    if false {
        let self_ticket = EndpointTicket::new(node.endpoint_addr());
        ceprintln!("{}", self_ticket);
    }

    // Wait until the user presses Ctrl-C to terminate the program:
    tokio::signal::ctrl_c().await?;

    // Close the connection and shut down the node:
    node.terminate().await?;

    Ok(())
}
