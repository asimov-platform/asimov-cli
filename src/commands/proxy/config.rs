// This is free and unencumbered software released into the public domain.

use super::ProxyConfigTarget;
use crate::StandardOptions;
use core::error::Error;

pub async fn config(
    app: &ProxyConfigTarget,
    format: &Option<String>,
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    use ProxyConfigTarget::*;
    match app {
        Bash | Zsh => match format.as_deref() {
            Some("sh") | _ => {
                println!("export OPENAI_API_BASE={}", "http://127.0.0.1:1920/v1");
                println!("export OPENAI_API_KEY={}", "sh");
            },
        },

        Dotenv => match format.as_deref() {
            Some("env") | _ => {
                println!("OPENAI_API_BASE={}", "http://127.0.0.1:1920/v1");
                println!("OPENAI_API_KEY={}", "dotenv");
            },
        },

        // See: <https://reference.langchain.com/python/langchain-openai/chat_models/base/ChatOpenAI>
        // See: <https://reference.langchain.com/javascript/langchain-openai/ChatOpenAI>
        Langchain => match format.as_deref() {
            Some("js") | Some("ts") => {
                print!("{}", include_str!("config/langchain.js"));
            },
            Some("py") | _ => {
                print!("{}", include_str!("config/langchain.py"));
            },
        },

        // See: <https://developers.llamaindex.ai/python/framework-api-reference/llms/openai_like/>
        Llamaindex => match format.as_deref() {
            Some("py") | _ => {
                print!("{}", include_str!("config/llamaindex.py"));
            },
        },

        // See: <https://docs.openhands.dev/openhands/usage/llms/custom-llm-configs>
        // See: <https://docs.openhands.dev/openhands/usage/v0/advanced/V0_configuration-options#llm-configuration>
        Openhands => match format.as_deref() {
            Some("env") => {
                println!("LLM_BASE_URL={}", "http://127.0.0.1:1920/v1");
                println!("LLM_API_KEY={}", "openhands");
                println!("LLM_MODEL={}", "openrouter/free");
            },
            Some("toml") | _ => {
                println!("[llm.asimov]");
                println!("base_url = \"{}\"", "http://127.0.0.1:1920/v1");
                println!("api_key = \"{}\"", "openhands");
                println!("model = \"{}\"", "openrouter/free");
            },
        },

        // See: <https://zed.dev/docs/reference/all-settings#language-models>
        Zed => match format.as_deref() {
            Some("json") | Some("jsonc") | _ => {
                use jsonc_parser::{ParseOptions, cst::CstRootNode};
                let cst = CstRootNode::parse(&"{}", &ParseOptions::default())?;
                let root = cst.object_value_or_set();
                root.object_value_or_set("language_models")
                    .object_value_or_set("openai_compatible")
                    .append("ASIMOV", zed_asimov_provider());
                println!("{}", cst.to_string());
            },
        },
    };
    Ok(())
}

pub(crate) fn zed_asimov_provider() -> jsonc_parser::cst::CstInputValue {
    use jsonc_parser::json;
    json!({
        "api_url": "http://127.0.0.1:1920/v1", // TODO: ASIMOV_PROXY_{HOST,PORT}
        "available_models": [], // TODO: https://github.com/asimov-datasets/openrouter.ai
    })
}
