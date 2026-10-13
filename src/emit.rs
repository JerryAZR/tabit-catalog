//! The emit side of the artifact: serde structs mirroring
//! tabit-config's `Provider`/`Model` (Deserialize-only there) with
//! `skip_serializing_if` for a minimal fragment. The round-trip
//! through `TabitConfig::from_toml_str` in the pipeline is what
//! keeps these two sides honest — schema drift fails the build, not
//! a user's boot.

use std::collections::BTreeMap;

use serde::Serialize;

/// A tabit providers fragment (the artifact body).
#[derive(Debug, Serialize)]
pub struct Fragment {
    pub providers: BTreeMap<String, Provider>,
}

#[derive(Debug, Serialize)]
pub struct Provider {
    pub name: String,
    pub base_url: String,
    /// The dialect ruling, a `WireApi` kebab-case spelling.
    pub api: String,
    pub api_key_env: String,
    pub models: Vec<Model>,
}

#[derive(Debug, Serialize)]
pub struct Model {
    pub id: String,
    /// Absent when identical to the id (the consumer's fallback).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Always true here: non-reasoning models are filtered out (the
    /// 80% ruling).
    pub reasoning: bool,
    /// Absent for text-only models (the consumer's default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Vec<&'static str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    /// All four legs, always — absent upstream legs are 0.0.
    pub cost: Cost,
    /// Always non-empty: the true vocabulary or the dialect's
    /// documented enum.
    pub thinking_levels: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Cost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

/// Serialize the fragment deterministically: provider and model
/// order come from the BTreeMaps (sorted), field order from the
/// struct declarations — identical input, identical bytes.
pub fn emit(fragment: &Fragment) -> Result<String, String> {
    toml::to_string_pretty(fragment).map_err(|err| format!("serialize: {err}"))
}
