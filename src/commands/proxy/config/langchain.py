from langchain_openai import ChatOpenAI

ASIMOV = ChatOpenAI(
    base_url="http://127.0.0.1:1920/v1",
    api_key="langchain",
    model="openrouter/free",
)

#print(ASIMOV.invoke("Tell me about Isaac Asimov"))
