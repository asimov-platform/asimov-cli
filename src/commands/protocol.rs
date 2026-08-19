// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use asimov_id::Id;
use asimov_protocol::Topic;
use clientele::{StandardOptions, crates::clap::Subcommand};

#[derive(Debug, Subcommand)]
pub enum ProtocolCommand {
    /// Accept and monitor inbound peer traffic
    #[clap(aliases = ["monitor", "pong"])]
    Accept {},

    /// Ping a peer node directly
    Ping {
        /// The handle to ping
        handle: Id,

        /// A peer ticket for bootstrapping
        #[arg(long)]
        ticket: Option<String>,
    },

    /// Connect to a peer node directly
    #[clap(alias = "hello")]
    Connect {
        /// The handle to connect to
        handle: Id,

        /// A peer ticket for bootstrapping
        #[arg(long)]
        ticket: Option<String>,
    },

    /// Publish a message to a gossip topic
    #[clap(aliases = ["pub", "send"])]
    Publish {
        /// The topic to publish to
        topic: Topic,

        /// The message to publish
        message: String,

        /// A peer ticket for bootstrapping
        #[arg(long)]
        ticket: Option<String>,
    },

    /// Resolve a handle into a set of peer IDs
    #[clap(aliases = ["lookup"])]
    Resolve {
        /// The handle to resolve
        handle: Id,
    },

    /// Subscribe to messages on a gossip topic
    #[clap(aliases = ["sub", "recv"])]
    Subscribe {
        /// The topic to publish to
        topic: Topic,

        /// A peer ticket for bootstrapping
        #[arg(long)]
        ticket: Option<String>,
    },
}

impl ProtocolCommand {
    pub async fn run(&self, flags: &StandardOptions) -> Result<(), BoxError> {
        use ProtocolCommand::*;
        match self {
            Accept {} => accept(flags).await,
            Ping { handle, ticket } => ping(handle, ticket, flags).await,
            Connect { handle, ticket } => connect(handle, ticket, flags).await,
            Publish {
                topic,
                message,
                ticket,
            } => publish(topic, message, ticket, flags).await,
            Resolve { handle } => resolve(handle, flags).await,
            Subscribe { topic, ticket } => subscribe(topic, ticket, flags).await,
        }
    }
}

mod accept;
pub use accept::*;

mod connect;
pub use connect::*;

mod ping;
pub use ping::*;

mod publish;
pub use publish::*;

mod resolve;
pub use resolve::*;

mod subscribe;
pub use subscribe::*;
