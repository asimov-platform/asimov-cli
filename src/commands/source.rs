// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use clientele::{StandardOptions, crates::clap::Subcommand};

#[derive(Debug, Subcommand)]
pub enum SourceCommand {
    /// Manage snapshots stored on disk
    #[cfg(feature = "source-snap")]
    Snap {
        #[clap(subcommand)]
        command: Option<SnapCommand>,

        #[clap(flatten)]
        args: SnapSaveArgs,
    },
}

impl SourceCommand {
    pub async fn run(self, flags: &StandardOptions) -> Result<(), BoxError> {
        use SourceCommand::*;
        match self {
            #[cfg(feature = "source-snap")]
            Snap { command, args } => {
                command
                    .unwrap_or(SnapCommand::Save { args })
                    .run(flags)
                    .await
            },
        }
    }
}

#[cfg(false)]
pub mod describe;

mod snap;
pub use snap::*;

#[cfg(test)]
mod tests {
    use super::*;
    use clientele::crates::clap::Parser;

    #[derive(Parser)]
    struct Command {
        #[command(subcommand)]
        command: SourceCommand,
    }

    #[test]
    fn snapshot_compaction_does_not_require_urls() {
        assert!(Command::try_parse_from(["source", "snap", "compact"]).is_ok());
    }
}
