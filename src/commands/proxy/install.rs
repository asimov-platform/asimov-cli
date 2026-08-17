// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use clap::ValueEnum;
use core::error::Error;
use jsonc_parser::{ParseOptions, cst::CstRootNode, json};

pub async fn install(
    apps: &[ProxyInstallApp],
    flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    let home_path = dirs::home_dir().expect("HOME should be set");
    for app in apps {
        match app {
            ProxyInstallApp::Zed => {
                if flags.verbose > 0 {
                    eprintln!("Configuring Zed...");
                }
                let zed_path = home_path.join(".config/zed/settings.json");
                if !zed_path.exists() {
                    eprintln!("error: {} not found.", zed_path.display());
                    return Ok(());
                }
                let zed_input =
                    std::fs::read_to_string(&zed_path).unwrap_or_else(|_| "{}".to_string());
                let cst = CstRootNode::parse(&zed_input, &ParseOptions::default())?;
                let root = cst.object_value_or_set();
                // See: https://zed.dev/docs/reference/all-settings#language-models
                if let Some(models) = root.get("language_models").and_then(|p| p.object_value())
                    && let Some(openai) = models
                        .get("openai_compatible")
                        .and_then(|p| p.object_value())
                {
                    let asimov_provider = json!({
                        "api_url": "http://127.0.0.1:1920/v1", // TODO: ASIMOV_PROXY_{HOST,PORT}
                        "available_models": [], // TODO: https://github.com/asimov-datasets/openrouter.ai
                    });
                    if let Some(asimov) = openai.get("ASIMOV") {
                        asimov.set_value(asimov_provider);
                    } else {
                        openai.append("ASIMOV", asimov_provider);
                    }
                }
                let zed_output = cst.to_string();
                //eprintln!("{}", &zed_output); // DEBUG
                std::fs::write(&zed_path, zed_output)?;
                if flags.verbose > 0 {
                    eprintln!("Configured Zed: {}", zed_path.display());
                }
            },
        };
    }
    Ok(())
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ProxyInstallApp {
    /// The Zed text editor (https://zed.dev).
    Zed,
}
