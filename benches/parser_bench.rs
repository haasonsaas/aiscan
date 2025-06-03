use aiscan::parser::FileParser;
use aiscan::patterns::PatternMatcher;
use criterion::{criterion_group, criterion_main, Criterion};
use std::path::PathBuf;

fn benchmark_tree_sitter_parsing(c: &mut Criterion) {
    let parser = FileParser::new().unwrap();
    let sample_code = r#"
import openai
from langchain.llms import ChatOpenAI
from autogen import AssistantAgent

client = openai.Client(api_key="sk-test")
llm = ChatOpenAI(model="gpt-4", temperature=0.7)
agent = AssistantAgent(name="assistant", llm_config={"model": "gpt-4"})

def process_request(prompt):
    response = client.chat.completions.create(
        model="gpt-4",
        messages=[{"role": "user", "content": prompt}]
    )
    return response.choices[0].message.content
"#;

    c.bench_function("tree_sitter_parse_python", |b| {
        b.iter(|| parser.parse_file(&PathBuf::from("test.py"), sample_code))
    });
}

fn benchmark_pattern_matching(c: &mut Criterion) {
    let matcher = PatternMatcher::new();
    let sample_code = r#"
import openai
from langchain.llms import ChatOpenAI
from autogen import AssistantAgent

OPENAI_API_KEY = "sk-test123"
ANTHROPIC_API_KEY = "ant-test456"

client = openai.Client(api_key=OPENAI_API_KEY)
llm = ChatOpenAI(model="gpt-4", temperature=0.7)
agent = AssistantAgent(name="assistant", llm_config={"model": "gpt-4"})

def process_request(prompt):
    response = client.chat.completions.create(
        model="gpt-4",
        messages=[{"role": "user", "content": prompt}]
    )
    return response.choices[0].message.content

# Multiple calls to test performance
for i in range(10):
    result = process_request(f"Test prompt {i}")
"#;

    c.bench_function("pattern_matching", |b| {
        b.iter(|| matcher.find_matches(&PathBuf::from("test.py"), sample_code))
    });
}

criterion_group!(
    benches,
    benchmark_tree_sitter_parsing,
    benchmark_pattern_matching
);
criterion_main!(benches);
