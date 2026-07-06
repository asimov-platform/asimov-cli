// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError};
use asimov_protocol::{EndpointTicket, Node, Ticket, Topic};
use color_print::ceprintln;

pub async fn send(
    topic: &String,
    message: &String,
    ticket: &Option<String>,
    _flags: &StandardOptions,
) -> Result<(), SysexitsError> {
    // Start a node and accept connections from peers:
    let mut node = Node::default().bind().await?.start().await?;
    node.online().await;

    // If a peer ticket was provided, use it for bootstrap:
    if let Some(ticket) = ticket {
        let peer_ticket = EndpointTicket::decode_string(ticket.as_ref()).unwrap();
        node.add_peer(peer_ticket.endpoint_addr().id);
    }

    // Print out the endpoint's ticket that allows connecting to it:
    let self_ticket = EndpointTicket::new(node.endpoint_addr());
    ceprintln!("{}", self_ticket);

    // Subscribe to the given topic and wait for a peer:
    let mut topic_subscription = node.subscribe_and_join(Topic::Handle(topic.into())).await?;
    ceprintln!("<s,g>✓</> Topic=<s>{:?}</>", topic_subscription);

    // Publish the given message to the topic:
    topic_subscription.publish(message.clone()).await?;

    // Wait until the user presses Ctrl-C to terminate the program:
    tokio::signal::ctrl_c().await?;

    // Close the connection and shut down the node:
    node.terminate().await?;

    Ok(())
}
