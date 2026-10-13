//! Pipeline tests over the fixtures: mapping semantics, invariants,
//! determinism, and the consumer round-trip.

use std::path::Path;

use tabit_catalog::{generate, overlay};

fn fixtures(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn run() -> tabit_catalog::Generated {
    let source = std::fs::read(fixtures("api.json")).unwrap();
    let overlay = overlay::load(&fixtures("overlay.toml")).unwrap();
    generate(&source, &overlay).expect("fixture pipeline succeeds")
}

fn artifact_providers(generated: &tabit_catalog::Generated) -> toml::Value {
    toml::from_str(&generated.artifact).unwrap()
}

/// The whole contract in one place: what shipped, with which
/// dialect, levels, costs, and endpoint rulings.
#[test]
fn semantics() {
    let generated = run();
    let providers = artifact_providers(&generated);
    let providers = providers.get("providers").unwrap();

    // The allowlist only: image-only and uncurated never appear.
    let ids: Vec<&str> = providers
        .as_table()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        ids,
        ["anthropic", "inference-test", "minimax", "openai", "zai"]
    );

    let anthropic = &providers["anthropic"];
    assert_eq!(
        anthropic["base_url"].as_str(),
        Some("https://api.anthropic.com")
    );
    assert_eq!(anthropic["api"].as_str(), Some("anthropic-messages"));
    assert_eq!(anthropic["api_key_env"].as_str(), Some("ANTHROPIC_API_KEY"));
    // claude-opus-4-6 and claude-haiku-4-2 shipped; the
    // non-reasoning sonnet and deprecated claude-2 dropped.
    let models = anthropic["models"].as_array().unwrap();
    assert_eq!(models.len(), 2);
    let model = |id: &str| {
        models
            .iter()
            .find(|model| model["id"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("{id} not shipped"))
            .clone()
    };
    let opus = model("claude-opus-4-6");
    assert_eq!(opus["input"], toml::Value::from(vec!["text", "image"]));
    // A true vocabulary: none->off, no unmapped names, so no
    // fallback and no "minimal" (not documented for the dialect).
    assert_eq!(
        opus["thinking_levels"],
        toml::Value::from(vec!["off", "low", "medium", "high"])
    );
    // Cost legs verbatim when present.
    assert_eq!(opus["cost"]["cache_write"].as_float(), Some(6.25));
    // The toggle-only haiku gets the dialect's documented enum.
    let haiku = model("claude-haiku-4-2");
    assert_eq!(
        haiku["thinking_levels"],
        toml::Value::from(vec!["off", "low", "medium", "high", "xhigh", "max"])
    );
    // Absent cost legs are 0.0, never absent fields.
    assert_eq!(haiku["cost"]["cache_read"].as_float(), Some(0.0));
    assert_eq!(haiku["cost"]["cache_write"].as_float(), Some(0.0));
    // Text-only models omit the input field (consumer default).
    assert!(haiku.get("input").is_none());

    let openai = &providers["openai"];
    assert_eq!(openai["api"].as_str(), Some("openai-responses"));
    let sol = &openai["models"].as_array().unwrap()[0];
    // "minimal" IS documented for openai dialects — kept.
    assert_eq!(
        sol["thinking_levels"],
        toml::Value::from(vec!["minimal", "low", "medium", "high"])
    );

    let zai = &providers["zai"];
    // The overlay's endpoint ruling replaces the source's
    // openai-compatible surface.
    assert_eq!(
        zai["base_url"].as_str(),
        Some("https://api.z.ai/api/anthropic")
    );
    assert_eq!(zai["api"].as_str(), Some("anthropic-messages"));
    let glm = zai["models"].as_array().unwrap();
    // "turbo" has no canonical mapping: dropped, reported, and the
    // rest of the true vocabulary kept (no fallback). The leading
    // null is the "off" level (`reasoning_effort: null`).
    assert_eq!(
        glm[0]["thinking_levels"],
        toml::Value::from(vec!["off", "low", "high", "max"])
    );
    // No reasoning_options at all: the dialect enum, which for
    // anthropic-messages excludes "minimal".
    assert_eq!(
        glm[1]["thinking_levels"],
        toml::Value::from(vec!["off", "low", "medium", "high", "xhigh", "max"])
    );

    let inference = &providers["inference-test"];
    // No dialect ruling in the overlay: npm (@ai-sdk/anthropic)
    // inferred it; base_url came from the source.
    assert_eq!(inference["api"].as_str(), Some("anthropic-messages"));
    assert_eq!(
        inference["base_url"].as_str(),
        Some("https://inference-test.example/anthropic/v1")
    );
    assert_eq!(
        inference["api_key_env"].as_str(),
        Some("INFERENCE_TEST_API_KEY")
    );
    // "ultra" unmapped and dropped; "medium"/"xhigh" kept.
    assert_eq!(
        inference["models"].as_array().unwrap()[0]["thinking_levels"],
        toml::Value::from(vec!["medium", "xhigh"])
    );

    let minimax = &providers["minimax"];
    assert_eq!(minimax["name"].as_str(), Some("MiniMax (curated)"));
    let m3 = &minimax["models"].as_array().unwrap()[0];
    // No cost upstream at all: all four legs 0.0. Video input is
    // not a tabit modality: text+image is what ships.
    assert_eq!(m3["cost"]["input"].as_float(), Some(0.0));
    assert_eq!(m3["cost"]["output"].as_float(), Some(0.0));
    assert_eq!(m3["cost"]["cache_read"].as_float(), Some(0.0));
    assert_eq!(m3["cost"]["cache_write"].as_float(), Some(0.0));
    assert_eq!(m3["input"], toml::Value::from(vec!["text", "image"]));

    // The report carries every drop with its reason.
    let dropped: Vec<String> = generated
        .report
        .dropped
        .iter()
        .map(|drop| format!("{}/{}", drop.provider, drop.model))
        .collect();
    assert_eq!(
        dropped,
        [
            "anthropic/claude-2",
            "anthropic/claude-3-5-sonnet",
            "openai/gpt-4o",
            "openai/gpt-6-alpha",
        ]
    );
    // And the unmapped level names.
    let levels: Vec<String> = generated
        .report
        .level_drops
        .iter()
        .map(|drop| format!("{}/{}:{}", drop.provider, drop.model, drop.level))
        .collect();
    assert_eq!(
        levels,
        ["inference-test/test-reasoner:ultra", "zai/glm-5.3:turbo"]
    );
}

/// Identical input produces identical bytes, stamp included.
#[test]
fn determinism() {
    assert_eq!(run().artifact, run().artifact);
    let first = run();
    let second = run();
    assert_eq!(first.artifact, second.artifact);
    assert_eq!(first.version, second.version);
    assert_eq!(first.sha256, second.sha256);
}

/// The consumer itself accepts the artifact (the pipeline already
/// gates on this; the assertion keeps the gate observable).
#[test]
fn consumer_schema_round_trip() {
    let generated = run();
    let config =
        tabit_config::TabitConfig::from_toml_str(&generated.artifact, Path::new("catalog.toml"))
            .unwrap();
    assert_eq!(config.providers.len(), 5);
    let (provider, model) = config.model("zai", "glm-5.3").unwrap();
    assert_eq!(provider.api, tabit_config::WireApi::AnthropicMessages);
    assert_eq!(model.thinking_levels, vec!["off", "low", "high", "max"]);
}

/// The pointer body: exactly the consumer's three fields, the
/// checksum over the exact artifact bytes.
#[test]
fn latest_toml_matches_artifact() {
    let generated = run();
    use sha2::Digest as _;
    let digest = sha2::Sha256::digest(generated.artifact.as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(generated.sha256, hex);
    assert_eq!(
        generated.latest,
        format!(
            "version = \"{}\"\nsha256 = \"{}\"\nmin_schema = 1\n",
            generated.version, generated.sha256
        )
    );
}

/// Every sync-invariant failure mode, each failing loudly with the
/// provider named.
#[test]
fn sync_failures() {
    let source = br#"{
        "nodefects": {
            "name": "No Dialect", "env": ["NODEFECTS_API_KEY"], "npm": "@ai-sdk/niche",
            "api": "https://nodefects.example/v1",
            "models": {"n1": {"id": "n1", "reasoning": true, "tool_call": true}}
        },
        "noendpoints": {
            "name": "No Endpoint", "env": ["NOENDPOINTS_API_KEY"], "npm": "@ai-sdk/openai-compatible",
            "models": {"e1": {"id": "e1", "reasoning": true, "tool_call": true}}
        },
        "nokeys": {
            "name": "No Keys", "npm": "@ai-sdk/openai-compatible",
            "api": "https://nokeys.example/v1",
            "models": {"k1": {"id": "k1", "reasoning": true, "tool_call": true}}
        },
        "lonely": {
            "name": "Lonely", "env": ["LONELY_API_KEY"], "npm": "@ai-sdk/openai-compatible",
            "api": "https://lonely.example/v1",
            "models": {
                "l1": {"id": "l1", "reasoning": false, "tool_call": true},
                "l2": {"id": "l2", "reasoning": true, "tool_call": false}
            }
        },
        "baaddialect": {
            "name": "Bad Dialect", "env": ["BAD_API_KEY"], "npm": "@ai-sdk/openai-compatible",
            "api": "https://bad.example/v1",
            "models": {"b1": {"id": "b1", "reasoning": true, "tool_call": true}}
        },
        "fine": {
            "name": "Fine", "env": ["FINE_API_KEY"], "npm": "@ai-sdk/openai-compatible",
            "api": "https://fine.example/v1",
            "models": {"f1": {"id": "f1", "reasoning": true, "tool_call": true}}
        },
        "twin": {
            "name": "Twin", "env": ["SHARED_API_KEY"], "npm": "@ai-sdk/openai-compatible",
            "api": "https://twin.example/v1",
            "models": {"t1": {"id": "t1", "reasoning": true, "tool_call": true}}
        },
        "triplet": {
            "name": "Triplet", "env": ["SHARED_API_KEY"], "npm": "@ai-sdk/openai-compatible",
            "api": "https://triplet.example/v1",
            "models": {"t2": {"id": "t2", "reasoning": true, "tool_call": true}}
        }
    }"#;
    let overlay: overlay::Overlay = toml::from_str(
        r#"
        [providers.ghost]
        [providers.nodefects]
        [providers.noendpoints]
        [providers.nokeys]
        [providers.lonely]
        [providers.baaddialect]
        api = "gemini-native"
        [providers.fine]
        [providers.twin]
        [providers.triplet]
        "#,
    )
    .unwrap();

    let err = generate(source, &overlay).unwrap_err().to_string();
    assert!(
        err.contains("`ghost` is allowlisted but absent from the source"),
        "{err}"
    );
    assert!(
        err.contains("`nodefects`: no dialect ruling and the npm package implies none"),
        "{err}"
    );
    assert!(
        err.contains("`noendpoints`: no base_url ruling and the source carries none"),
        "{err}"
    );
    assert!(
        err.contains("`nokeys`: no api_key_env ruling and the source carries no env"),
        "{err}"
    );
    assert!(
        err.contains("`lonely`: zero models survived filtering"),
        "{err}"
    );
    assert!(err.contains("not a reasoning model"), "{err}");
    assert!(err.contains("no tool calling"), "{err}");
    assert!(
        err.contains("`baaddialect`: unknown dialect `gemini-native`"),
        "{err}"
    );
    // One key must not light up several providers' models.
    assert!(
        err.contains("api_key_env `SHARED_API_KEY` is shared by `triplet`, `twin`"),
        "{err}"
    );
}
