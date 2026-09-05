// This is free and unencumbered software released into the public domain.

use crate::BoxError;
use clientele::{StandardOptions, crates::clap::Subcommand};

#[derive(Debug, Subcommand)]
pub enum ProxyCommand {
    /// Run an OpenAI-compatible endpoint at <http://127.0.0.1:1920>.
    ///
    /// Requests are proxied to enabled providers (currently always OpenRouter).
    ///
    /// Reads OPENROUTER_API_KEY for the OpenRouter API key (required).
    ///
    /// Reads ASIMOV_PROXY_PORT for the port to bind to (default: 1920).
    ///
    /// Reads ASIMOV_PROXY_BIND for the address to bind to (default: 127.0.0.1).
    ///
    /// Reads ASIMOV_PROXY_LOG_FILE for a file to append request and
    /// response bodies to (optional).
    ///
    /// Honors the conventional https_proxy/HTTPS_PROXY, all_proxy/ALL_PROXY,
    /// and no_proxy/NO_PROXY environment variables for reaching upstream
    /// through an HTTP(S) or SOCKS5 proxy.
    #[clap(aliases = ["run"])]
    Serve {
        #[clap(flatten)]
        args: ProxyServeArgs,
    },

    /// Print the proxy URL.
    #[clap(aliases = ["link"])]
    Url {},

    /// Print the proxy host.
    Host {},

    /// Print the proxy port.
    Port {},

    /// List available models.
    #[cfg(feature = "unstable")]
    Models {
        /// The output format.
        /// [default: list]
        /// [possible values: csv, json, list, md, tsv]
        #[clap(short, long)]
        format: Option<String>,
    },

    /// Show configuration for using the proxy endpoint.
    Config {
        /// The target application to configure.
        app: ProxyConfigTarget,

        /// The output format.
        /// [default: auto]
        /// [possible values: env, js, json, py, toml, sh, ts]
        #[clap(short, long)]
        format: Option<String>,
    },

    /// Configure applications to use the proxy endpoint.
    Install {
        /// The target applications to configure.
        apps: Vec<ProxyInstallTarget>,
    },
}

impl ProxyCommand {
    pub async fn run(self, flags: &StandardOptions) -> Result<(), BoxError> {
        use ProxyCommand::*;
        match self {
            Serve { args } => serve(args, flags).await,
            Url {} => url(flags).await,
            Host {} => host(flags).await,
            Port {} => port(flags).await,
            #[cfg(feature = "unstable")]
            Models { format } => models(format, flags).await,
            Config { app, format } => config(app, format, flags).await,
            Install { apps } => install(apps, flags).await,
        }
    }
}

mod config;
pub use config::*;

mod host;
pub use host::*;

mod install;
pub use install::*;

#[cfg(feature = "unstable")]
mod models;
#[cfg(feature = "unstable")]
pub use models::*;

mod port;
pub use port::*;

mod serve;
pub use serve::*;

mod url;
pub use url::*;
