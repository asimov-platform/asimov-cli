// This is free and unencumbered software released into the public domain.

use asimov_protocol::Topic;
use clientele::{StandardOptions, crates::clap::Subcommand};
use core::error::Error;
use std::string::String;

#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    /// Send a message to a peer
    Send {
        /// The recipient for the message
        recipient: Topic,

        /// The message to send
        message: String,

        /// A peer ticket for bootstrapping
        #[arg(long)]
        ticket: Option<String>,
    },
}

impl MessageCommand {
    pub async fn run(&self, flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
        use MessageCommand::*;
        match self {
            Send {
                recipient,
                message,
                ticket,
            } => send(recipient, message, ticket, flags).await,
        }
    }
}

mod send;
pub use send::*;
