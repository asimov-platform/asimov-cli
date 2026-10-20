// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use asimov_module::ModuleName;
use clientele::{StandardOptions, crates::clap::Subcommand};

#[derive(Debug, Subcommand)]
pub enum SourceCommand {
    /// List resources using enabled modules, or ASIMOV_RELAY_URL as fallback.
    #[clap(aliases = ["dir", "ls"])]
    List {
        #[clap(flatten)]
        args: SourceListArgs,
    },

    /// Read a resource specified by a URL, utilizing enabled modules
    Read {
        #[clap(long, short = 'M')]
        module: Option<ModuleName>,

        #[arg(required = true)]
        urls: Vec<String>,
    },

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
            List { args } => list(args, flags).await,

            Read { module, urls } => read(urls, module, flags).await,

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

mod list;
pub use list::{SourceListArgs, list};

mod read;
pub use read::*;

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
    fn source_operations_require_urls() {
        for operation in ["list", "ls", "read"] {
            let error = Command::try_parse_from(["source", operation])
                .err()
                .expect("URL is required");
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument
            );
            assert!(Command::try_parse_from(["source", operation, "https://example.com"]).is_ok());
        }
        assert!(Command::try_parse_from(["source", "snap", "compact"]).is_ok());
    }

    #[test]
    fn list_parses_shared_options() {
        let Command {
            command:
                SourceCommand::List {
                    args:
                        SourceListArgs {
                            cache,
                            timing,
                            filtering,
                            urls,
                            ..
                        },
                },
        } = Command::try_parse_from([
            "test",
            "ls",
            "--max-age",
            "1h",
            "--deadline",
            "0s",
            "--jev",
            "The name is Ukrainian",
            "--json",
            "@id,name",
            "--jq",
            ".name",
            "https://example.com/",
        ])
        .unwrap()
        else {
            panic!("expected source list");
        };
        assert_eq!(cache.max_age_option().as_deref(), Some("--max-age=1h"));
        assert_eq!(timing.deadline_option().as_deref(), Some("--deadline=0s"));
        assert_eq!(filtering.jev.as_deref(), Some("The name is Ukrainian"));
        assert_eq!(filtering.json.unwrap(), ["@id", "name"]);
        assert_eq!(filtering.jq.as_deref(), Some(".name"));
        assert_eq!(urls, ["https://example.com/"]);
    }
}
