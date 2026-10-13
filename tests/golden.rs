//! The golden artifact: the fixture pipeline's exact output. The
//! TOML shape IS the contract (the consumer parses this byte stream
//! after download), so the full body is pinned here; a formatting
//! or schema change that survives the round-trip gate must be a
//! deliberate one, made here.

use std::path::Path;

use tabit_catalog::{generate, overlay};

fn run() -> tabit_catalog::Generated {
    let source =
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/api.json"))
            .unwrap();
    let overlay =
        overlay::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/overlay.toml"))
            .unwrap();
    generate(&source, &overlay).expect("fixture pipeline succeeds")
}

#[test]
fn golden_artifact() {
    assert_eq!(run().artifact, GOLDEN);
}

const GOLDEN: &str = r#"[providers.anthropic]
name = "Anthropic"
base_url = "https://api.anthropic.com"
api = "anthropic-messages"
api_key_env = "ANTHROPIC_API_KEY"

[[providers.anthropic.models]]
id = "claude-haiku-4-2"
name = "Claude Haiku 4.2"
reasoning = true
context_window = 200000
max_tokens = 32000
thinking_levels = [
    "off",
    "low",
    "medium",
    "high",
    "xhigh",
    "max",
]

[providers.anthropic.models.cost]
input = 1.0
output = 5.0
cache_read = 0.0
cache_write = 0.0

[[providers.anthropic.models]]
id = "claude-opus-4-6"
name = "Claude Opus 4.6"
reasoning = true
input = [
    "text",
    "image",
]
context_window = 200000
max_tokens = 64000
thinking_levels = [
    "off",
    "low",
    "medium",
    "high",
]

[providers.anthropic.models.cost]
input = 5.0
output = 25.0
cache_read = 0.5
cache_write = 6.25

[providers.inference-test]
name = "Inference Test"
base_url = "https://inference-test.example/anthropic/v1"
api = "anthropic-messages"
api_key_env = "INFERENCE_TEST_API_KEY"

[[providers.inference-test.models]]
id = "test-reasoner"
reasoning = true
context_window = 64000
max_tokens = 16000
thinking_levels = [
    "medium",
    "xhigh",
]

[providers.inference-test.models.cost]
input = 0.0
output = 0.0
cache_read = 0.0
cache_write = 0.0

[providers.minimax]
name = "MiniMax (curated)"
base_url = "https://api.minimax.io/anthropic"
api = "anthropic-messages"
api_key_env = "MINIMAX_API_KEY"

[[providers.minimax.models]]
id = "MiniMax-M3"
reasoning = true
input = [
    "text",
    "image",
]
context_window = 512000
max_tokens = 128000
thinking_levels = [
    "off",
    "low",
    "medium",
    "high",
    "xhigh",
    "max",
]

[providers.minimax.models.cost]
input = 0.0
output = 0.0
cache_read = 0.0
cache_write = 0.0

[providers.openai]
name = "OpenAI"
base_url = "https://api.openai.com/v1"
api = "openai-responses"
api_key_env = "OPENAI_API_KEY"

[[providers.openai.models]]
id = "gpt-6-sol"
name = "GPT-6 Sol"
reasoning = true
input = [
    "text",
    "image",
]
context_window = 400000
max_tokens = 128000
thinking_levels = [
    "minimal",
    "low",
    "medium",
    "high",
]

[providers.openai.models.cost]
input = 1.25
output = 10.0
cache_read = 0.125
cache_write = 0.0

[providers.zai]
name = "Z.AI"
base_url = "https://api.z.ai/api/anthropic"
api = "anthropic-messages"
api_key_env = "ZHIPU_API_KEY"

[[providers.zai.models]]
id = "glm-5.3"
name = "GLM-5.3"
reasoning = true
context_window = 1000000
max_tokens = 128000
thinking_levels = [
    "off",
    "low",
    "high",
    "max",
]

[providers.zai.models.cost]
input = 0.6
output = 2.2
cache_read = 0.0
cache_write = 0.0

[[providers.zai.models]]
id = "glm-5.4"
name = "GLM-5.4"
reasoning = true
context_window = 200000
max_tokens = 100000
thinking_levels = [
    "off",
    "low",
    "medium",
    "high",
    "xhigh",
    "max",
]

[providers.zai.models.cost]
input = 1.0
output = 4.0
cache_read = 0.1
cache_write = 1.0
"#;
