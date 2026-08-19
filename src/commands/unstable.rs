// This is free and unencumbered software released into the public domain.

use clientele::crates::clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum UnstableCommand {
    // /// Add, remove, and resolve handles and their associated endpoints
    // #[cfg(feature = "handle")]
    // #[clap(subcommand)]
    // Handle(HandleCommand),

    // /// Message other peers
    // #[cfg(feature = "message")]
    // #[clap(subcommand)]
    // Message(MessageCommand),

    // /// Package development commands
    // #[cfg(feature = "package")]
    // #[clap(subcommand)]
    // Package(PackageCommand),

    // /// Low-level protocol commands
    // #[cfg(feature = "protocol")]
    // #[clap(subcommand)]
    // Protocol(ProtocolCommand),

    // /// TBD
    // #[cfg(feature = "search")]
    // Search {
    //     #[clap(long, short = 'M')]
    //     module: Option<ModuleName>,

    //     prompt: String,
    // },
}

// #[cfg(feature = "ask")]
// Ask {
//     module,
//     model,
//     input,
// } => {
//     let input = if let Some(input) = input {
//         input.clone()
//     } else {
//         use std::io::Read;
//         let mut buf = String::new();
//         if std::io::stdin().read_to_string(&mut buf).is_err() {
//             return EX_IOERR;
//         };
//         buf
//     };
//     commands::ask::ask(input, module, model, flags)
//         .await
//         .map(|_| EX_OK)
// },

// #[cfg(feature = "describe")]
// Describe {
//     module,
//     output,
//     urls,
// } => commands::describe::describe(urls, module, output, flags)
//     .await
//     .map(|_| EX_OK),

// #[cfg(feature = "handle")]
// Handle(command) => command.run(flags).await.map_err(sysexits).map(|_| EX_OK),

// #[cfg(feature = "index")]
// Index { module, urls } => commands::index::index(urls, module, flags)
//     .await
//     .map(|_| EX_OK),

// #[cfg(feature = "list")]
// List {
//     module,
//     limit,
//     output,
//     urls,
// } => commands::list::list(urls, module, limit, output, flags)
//     .await
//     .map(|_| EX_OK),

// #[cfg(feature = "message")]
// Message(command) => command.run(flags).await.map_err(sysexits).map(|_| EX_OK),

// #[cfg(feature = "package")]
// Package(command) => command.run(flags).await.map_err(sysexits).map(|_| EX_OK),

// #[cfg(feature = "protocol")]
// Protocol(command) => command.run(flags).await.map_err(sysexits).map(|_| EX_OK),

// #[cfg(feature = "search")]
// Search { module, prompt } => commands::search::search(prompt, module, flags)
//     .await
//     .map(|_| EX_OK),

#[cfg(feature = "agent")]
pub mod agent;

#[cfg(feature = "cache")]
pub mod cache;

#[cfg(feature = "construct")]
pub mod construct;

#[cfg(feature = "dataset")]
pub mod dataset;

#[cfg(feature = "flow")]
pub mod flow;

#[cfg(feature = "graph")]
pub mod graph;

#[cfg(feature = "handle")]
pub mod handle;

#[cfg(feature = "message")]
pub mod message;

#[cfg(feature = "package")]
pub mod package;

#[cfg(feature = "protocol")]
pub mod protocol;

#[cfg(feature = "vault")]
pub mod vault;
