import { ChatOpenAI } from "@langchain/openai";

const ASIMOV = new ChatOpenAI({
  configuration: {
    baseURL: "http://127.0.0.1:1920/v1"
  },
  apiKey: "langchain",
  model: "openrouter/free",
});

//console.log(await ASIMOV.invoke("Tell me about Isaac Asimov"));
