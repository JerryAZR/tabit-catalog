//! The curated overlay: the catalog's editorial line. Presence in
//! `providers` IS the allowlist (PROVIDERS.md's 80% ruling: the
//! common providers only; niche ones are user config). Each entry
//! rules the dialect and, where the upstream's endpoint follows
//! ai-sdk rather than tabit conventions, the base URL. Data, not
//! code — the Rule 1 spirit applied to the generator itself.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Overlay {
    pub providers: BTreeMap<String, Provider>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Provider {
    /// The dialect ruling: "anthropic-messages", "openai-responses",
    /// or "openai-completions". May be omitted only when the
    /// upstream npm package implies the dialect
    /// (see [`crate::map::infer_api`]).
    pub api: Option<String>,
    /// tabit-convention base URL (the engines append their request
    /// paths to it). When omitted, the upstream's `api` is used.
    pub base_url: Option<String>,
    /// Overrides the upstream's primary environment-variable name.
    pub api_key_env: Option<String>,
    /// Overrides the upstream's display name.
    pub name: Option<String>,
    /// Provenance note for humans; never emitted.
    pub note: Option<String>,
}

pub fn load(path: &Path) -> Result<Overlay, String> {
    let raw = std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    toml::from_str(&raw).map_err(|err| format!("{}: {err}", path.display()))
}
