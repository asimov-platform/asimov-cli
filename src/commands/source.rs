// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use asimov_module::ModuleName;
use clientele::{StandardOptions, crates::clap::Subcommand, options::sort::SortKeys};

#[derive(Debug, Subcommand)]
pub enum SourceCommand {
    /// Fetch a resource from a URL, utilizing enabled modules.
    #[clap(aliases = ["extract", "get", "import", "ingest", "parse"])]
    Fetch {
        #[clap(flatten)]
        args: fetch::SourceFetchArgs,
    },

    /// List resources from a collection URL, utilizing enabled modules.
    #[clap(aliases = ["dir", "ls"])]
    List {
        /// The collection URL(s) to examine.
        urls: Vec<String>,

        /// The specific module to use.
        #[clap(long, short = 'M')]
        module: Option<ModuleName>,

        /// Sort resources by the specified keys. (Prefix a key with `-` for descending order.)
        #[clap(long, aliases = ["sort-by", "order", "order-by"], value_name = "[+|-]KEY,...", allow_hyphen_values = true)]
        sort: Option<SortKeys>,

        /// The index offset of the first output.
        #[clap(value_name = "INDEX", long, default_value = "0")]
        offset: Option<usize>,

        /// The maximum count of outputs [default: none].
        #[arg(value_name = "COUNT", short = 'n', long)]
        limit: Option<usize>,

        /// The output format.
        #[arg(value_name = "FORMAT", short = 'o', long)]
        output: Option<String>, // TODO: OutputFormat, default_value = "jsonl"
    },

    /// Read a resource specified by a URL, utilizing enabled modules
    Read {
        #[clap(long, short = 'M')]
        module: Option<ModuleName>,

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

impl Default for SourceCommand {
    fn default() -> Self {
        SourceCommand::Fetch {
            args: fetch::SourceFetchArgs::default(),
        }
    }
}

impl SourceCommand {
    pub async fn run(self, flags: &StandardOptions) -> Result<(), BoxError> {
        use SourceCommand::*;
        match self {
            Fetch { args } => fetch(args, flags).await,

            List {
                urls,
                module,
                sort,
                offset,
                limit,
                output,
            } => list(urls, module, sort, offset, limit, output, flags).await,

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

mod fetch;
pub use fetch::*;

mod list;
pub use list::list;

mod read;
pub use read::*;

mod snap;
pub use snap::*;
