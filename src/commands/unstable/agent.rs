// This is free and unencumbered software released into the public domain.

use asimov_module::ModuleName;
use clientele::crates::clap::Subcommand;

#[derive(Debug, Subcommand)]
enum Command {
    /// Prompt an LLM with text input
    Ask {
        #[clap(long, short = 'M')]
        module: Option<ModuleName>,

        #[clap(long, short = 'm')]
        model: Option<String>,

        input: Option<String>,
    },

    /// TBD
    #[clap(aliases = ["summarize", "tldr"])]
    Describe {
        #[clap(long, short = 'M')]
        module: Option<ModuleName>,

        /// The output format.
        #[arg(value_name = "FORMAT", short = 'o', long)]
        output: Option<String>,

        urls: Vec<String>,
    },
}

pub mod ask;

pub mod search;
