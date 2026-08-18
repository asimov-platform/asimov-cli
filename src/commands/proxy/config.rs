// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use clap::ValueEnum;
use core::error::Error;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ProxyConfigTarget {
    /// Aider (https://aider.chat).
    Aider,

    /// Bash (https://gnu.org/software/bash/).
    Bash,

    /// Claude Code (https://claude.com/product/claude-code).
    #[cfg(feature = "unstable")]
    ClaudeCode,

    /// Cline (https://cline.bot).
    #[cfg(feature = "unstable")]
    Cline,

    /// .env (https://github.com/motdotla/dotenv).
    Dotenv,

    /// Goose (https://goose-docs.ai).
    Goose,

    /// LangChain (https://langchain.com).
    Langchain,

    /// LiteLLM (https://litellm.ai).
    Litellm,

    /// LlamaIndex (https://llamaindex.ai).
    Llamaindex,

    /// OpenCode (https://opencode.ai).
    Opencode,

    /// OpenHands, fka OpenDevin (https://openhands.dev).
    Openhands,

    /// Pi (https://pi.dev).
    Pi,

    /// PowerShell (https://microsoft.com/powershell).
    Powershell,

    /// Zed (https://zed.dev).
    Zed,

    /// Zsh (https://zsh.sourceforge.io).
    Zsh,
}

pub async fn config(
    app: &ProxyConfigTarget,
    format: &Option<String>,
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    use ProxyConfigTarget::*;
    let base_url = "http://127.0.0.1:1920/v1"; // TODO
    let default_model = "openrouter/free";
    match app {
        // See: <https://aider.chat/docs/llms/openai-compat.html>
        Aider => match format.as_deref() {
            None => {
                let export = if cfg!(windows) { "setx" } else { "export" };
                println!("{export} {}={}", "OPENAI_API_BASE", base_url);
                println!("{export} {}={}", "OPENAI_API_KEY", "aider");
            },
            Some(_) => {},
        },

        Bash | Zsh => match format.as_deref() {
            Some("export") | None => {
                println!("export {}={}", "OPENAI_API_BASE", base_url);
                println!("export {}={}", "OPENAI_API_KEY", "sh");
            },
            Some(_) => {},
        },

        #[cfg(feature = "unstable")]
        ClaudeCode => todo!(), // TODO

        // See: <https://docs.cline.bot/running-models-locally/overview>
        #[cfg(feature = "unstable")]
        Cline => todo!(), // TODO: no support for arbitrary endpoints?

        Dotenv => match format.as_deref() {
            Some("env") | None => {
                println!("{}={}", "OPENAI_API_BASE", base_url);
                println!("{}={}", "OPENAI_API_KEY", "dotenv");
            },
            Some(_) => {},
        },

        // See: <https://goose-docs.ai/docs/getting-started/providers/#configure-custom-provider>
        // See: <https://github.com/aaif-goose/goose/blob/main/crates/goose-providers/src/declarative.rs#L112>
        // See: <https://github.com/aaif-goose/goose/blob/main/crates/goose-providers/src/openai_compatible.rs>
        Goose => match format.as_deref() {
            Some("json") => {
                print!("{}", include_str!("config/goose.json"));
            },
            Some("jsonc") | None => {
                print!(
                    "// {}\n{}",
                    "~/.config/goose/custom_providers/asimov.json",
                    include_str!("config/goose.json")
                );
            },
            Some(_) => {},
        },

        // See: <https://reference.langchain.com/python/langchain-openai/chat_models/base/ChatOpenAI>
        // See: <https://reference.langchain.com/javascript/langchain-openai/ChatOpenAI>
        Langchain => match format.as_deref() {
            Some("py") | None => {
                print!("{}", include_str!("config/langchain.py"));
            },
            Some("js") | Some("ts") => {
                print!("{}", include_str!("config/langchain.js"));
            },
            Some(_) => {},
        },

        // See: <https://docs.litellm.ai/docs/contributing/adding_openai_compatible_providers>
        Litellm => match format.as_deref() {
            Some("json") => {
                print!("{}", include_str!("config/litellm.json"));
            },
            Some("jsonc") | None => {
                print!(
                    "// {}\n{}",
                    "litellm/llms/openai_like/providers.json",
                    include_str!("config/litellm.json")
                );
            },
            Some(_) => {},
        },

        // See: <https://developers.llamaindex.ai/python/framework-api-reference/llms/openai_like/>
        Llamaindex => match format.as_deref() {
            Some("py") | None => {
                print!("{}", include_str!("config/llamaindex.py"));
            },
            Some(_) => {},
        },

        // See: <https://opencode.ai/docs/providers/#custom-provider>
        Opencode => match format.as_deref() {
            Some("json") => {
                print!("{}", include_str!("config/opencode.json"));
            },
            Some("jsonc") | None => {
                print!(
                    "// {}\n{}",
                    "~/.config/opencode/opencode.json",
                    include_str!("config/opencode.json")
                );
            },
            Some(_) => {},
        },

        // See: <https://docs.openhands.dev/openhands/usage/llms/custom-llm-configs>
        // See: <https://docs.openhands.dev/openhands/usage/v0/advanced/V0_configuration-options#llm-configuration>
        Openhands => match format.as_deref() {
            Some("env") => {
                println!("{}={}", "LLM_BASE_URL", base_url);
                println!("{}={}", "LLM_API_KEY", "openhands");
                println!("{}={}", "LLM_MODEL", default_model);
            },
            Some("toml") | None => {
                println!("[llm.asimov]");
                println!("base_url = \"{}\"", base_url);
                println!("api_key = \"{}\"", "openhands");
                println!("model = \"{}\"", default_model);
            },
            Some(_) => {},
        },

        Powershell => match format.as_deref() {
            Some("dotnet") | None => {
                println!(
                    r#"[Environment]::SetEnvironmentVariable("{}", "{}", "User")"#,
                    "OPENAI_API_BASE", base_url
                );
                println!(
                    r#"[Environment]::SetEnvironmentVariable("{}", "{}", "User")"#,
                    "OPENAI_API_KEY", "powershell"
                );
            },
            Some("set") | Some("setx") => {
                println!("setx {}={}", "OPENAI_API_BASE", base_url);
                println!("setx {}={}", "OPENAI_API_KEY", "powershell");
            },
            Some(_) => {},
        },

        // See: <https://pi.dev/docs/latest/providers#custom-providers>
        // See: <https://pi.dev/docs/latest/models>
        Pi => match format.as_deref() {
            Some("json") => {
                print!("{}", include_str!("config/pi.json"));
            },
            Some("jsonc") | None => {
                print!(
                    "// {}\n{}",
                    "~/.pi/agent/models.json",
                    include_str!("config/pi.json")
                );
            },
            Some(_) => {},
        },

        // See: <https://zed.dev/docs/reference/all-settings#language-models>
        Zed => match format.as_deref() {
            Some("json") => {
                println!("{}", zed_asimov_config()?);
            },
            Some("jsonc") | None => {
                println!(
                    "// {}\n{}",
                    "~/.config/zed/settings.json",
                    zed_asimov_config()?
                );
            },
            Some(_) => {},
        },
    };
    Ok(())
}

pub(crate) fn zed_asimov_config() -> Result<String, jsonc_parser::errors::ParseError> {
    use jsonc_parser::{ParseOptions, cst::CstRootNode};
    let cst = CstRootNode::parse(&"{}", &ParseOptions::default())?;
    let root = cst.object_value_or_set();
    root.object_value_or_set("language_models")
        .object_value_or_set("openai_compatible")
        .append("ASIMOV", zed_asimov_provider());
    Ok(cst.to_string())
}

pub(crate) fn zed_asimov_provider() -> jsonc_parser::cst::CstInputValue {
    use jsonc_parser::json;
    json!({
        "api_url": "http://127.0.0.1:1920/v1", // TODO: ASIMOV_PROXY_{HOST,PORT}
        "available_models": [], // TODO: https://github.com/asimov-datasets/openrouter.ai
    })
}
