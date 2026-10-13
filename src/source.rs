//! The models.dev `api.json` payload: a lenient read-side schema.
//! Only the fields the catalog consumes are named; everything else
//! in the upstream evolves freely and is ignored here.

use std::collections::BTreeMap;

use serde::Deserialize;

/// The whole payload: providers keyed by their upstream id.
#[derive(Debug, Deserialize)]
pub struct Api(pub BTreeMap<String, Provider>);

#[derive(Debug, Deserialize)]
pub struct Provider {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    /// Environment variables the API key can live in, primary
    /// first (the catalog ships the first).
    #[serde(default)]
    pub env: Option<Vec<String>>,
    /// The ai-sdk package the upstream associates with this
    /// provider — a dialect hint for the overlay's fallback
    /// inference.
    #[serde(default)]
    pub npm: Option<String>,
    /// The upstream's endpoint. It follows ai-sdk conventions, not
    /// tabit's (the engines append their own paths), so the
    /// overlay rules the base_url the catalog ships.
    #[serde(default)]
    pub api: Option<String>,
    #[serde(default)]
    pub models: BTreeMap<String, Model>,
}

#[derive(Debug, Deserialize)]
pub struct Model {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub reasoning: Option<bool>,
    #[serde(default)]
    pub tool_call: Option<bool>,
    /// Lifecycle marker: "alpha" / "beta" / "deprecated".
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub modalities: Option<Modalities>,
    #[serde(default)]
    pub cost: Option<Cost>,
    #[serde(default)]
    pub limit: Option<Limit>,
    #[serde(default)]
    pub reasoning_options: Option<Vec<ReasoningOption>>,
}

#[derive(Debug, Deserialize)]
pub struct Modalities {
    #[serde(default)]
    pub input: Option<Vec<String>>,
}

/// USD per million tokens, any leg possibly absent upstream.
#[derive(Debug, Deserialize)]
pub struct Cost {
    #[serde(default)]
    pub input: Option<f64>,
    #[serde(default)]
    pub output: Option<f64>,
    #[serde(default)]
    pub cache_read: Option<f64>,
    #[serde(default)]
    pub cache_write: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct Limit {
    #[serde(default)]
    pub context: Option<u64>,
    #[serde(default)]
    pub output: Option<u64>,
}

/// One reasoning-options entry. Only `type = "effort"` entries
/// carry a level vocabulary (`values`) — and that vocabulary is the
/// provider's *true* level list, the resolution of PROVIDERS.md's
/// level-fidelity question. Other kinds (`toggle`, `budget_tokens`)
/// carry no vocabulary for us.
#[derive(Debug, Deserialize)]
pub struct ReasoningOption {
    #[serde(rename = "type")]
    pub kind: String,
    /// A `null` element is a level name: the OpenAI convention's
    /// `reasoning_effort: null` — the explicit "off" level.
    #[serde(default)]
    pub values: Option<Vec<Option<String>>>,
}

pub fn parse(bytes: &[u8]) -> Result<Api, String> {
    serde_json::from_slice(bytes).map_err(|err| err.to_string())
}
