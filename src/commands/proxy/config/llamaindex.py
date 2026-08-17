from llama_index.core import Settings
from llama_index.llms.openai_like import OpenAILike

Settings.llm = OpenAILike(
    api_base="http://127.0.0.1:1920/v1",
    api_key="llamaindex",
    model="openrouter/free",
    is_chat_model=True,
)

#print(Settings.llm.complete("Tell me about Isaac Asimov"))
