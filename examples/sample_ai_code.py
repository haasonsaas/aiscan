import os
import openai
from langchain.llms import ChatOpenAI
from autogen import AssistantAgent

# Hardcoded API key (security issue!)
OPENAI_API_KEY = "sk-test123456789"

# Better practice - from environment
anthropic_key = os.getenv("ANTHROPIC_API_KEY")

# OpenAI client
client = openai.Client(api_key=OPENAI_API_KEY)

# LangChain
llm = ChatOpenAI(model="gpt-4", temperature=0.7)

# Autogen agent
assistant = AssistantAgent(
    name="assistant",
    llm_config={
        "model": "gpt-4",
        "api_key": OPENAI_API_KEY,
    }
)

def process_user_input(user_input):
    # No input validation - security risk!
    response = client.chat.completions.create(
        model="gpt-4",
        messages=[
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": user_input}
        ]
    )
    return response.choices[0].message.content

# Expensive model without rate limiting
def analyze_document(doc):
    return llm.invoke(f"Analyze this document: {doc}")