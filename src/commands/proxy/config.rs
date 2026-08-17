// This is free and unencumbered software released into the public domain.

use super::ProxyConfigTarget;
use crate::StandardOptions;
use core::error::Error;

pub async fn config(
    app: &ProxyConfigTarget,
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    use ProxyConfigTarget::*;
    match app {
        Bash | Zsh => {
            println!("export OPENAI_API_BASE={}", "http://127.0.0.1:1920/v1");
            println!("export OPENAI_API_KEY={}", "sh");
        },

        Dotenv => {
            println!("OPENAI_API_BASE={}", "http://127.0.0.1:1920/v1");
            println!("OPENAI_API_KEY={}", "dotenv");
        },

        Langchain => {
            // See: https://reference.langchain.com/python/langchain-openai/chat_models/base/ChatOpenAI
            print!("{}", include_str!("config/langchain.py"));
        },

        Llamaindex => {
            // See: https://developers.llamaindex.ai/python/framework-api-reference/llms/openai_like/
            print!("{}", include_str!("config/llamaindex.py"));
        },

        Openhands => {
            // See: https://docs.openhands.dev/openhands/usage/llms/custom-llm-configs
            // See: https://docs.openhands.dev/openhands/usage/v0/advanced/V0_configuration-options#llm-configuration
            if true {
                println!("[llm.asimov]");
                println!("base_url = \"{}\"", "http://127.0.0.1:1920/v1");
                println!("api_key = \"{}\"", "openhands");
                println!("model = \"{}\"", "openrouter/free");
            } else {
                println!("LLM_BASE_URL={}", "http://127.0.0.1:1920/v1");
                println!("LLM_API_KEY={}", "openhands");
                println!("LLM_MODEL={}", "openrouter/free");
            }
        },

        Zed => {
            todo!() // TODO
        },
    };
    Ok(())
}
