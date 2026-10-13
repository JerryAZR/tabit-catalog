//! Source → artifact mapping: the allowlist walk, the model
//! filters, dialect resolution, and the thinking-level vocabulary.

use crate::emit;
use crate::overlay;
use crate::source;

/// The dialects the artifact can rule — tabit's `WireApi` kebab-case
/// spellings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Api {
    AnthropicMessages,
    OpenaiResponses,
    OpenaiCompletions,
}

impl Api {
    pub fn parse(spelling: &str) -> Option<Self> {
        Some(match spelling {
            "anthropic-messages" => Self::AnthropicMessages,
            "openai-responses" => Self::OpenaiResponses,
            "openai-completions" => Self::OpenaiCompletions,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AnthropicMessages => "anthropic-messages",
            Self::OpenaiResponses => "openai-responses",
            Self::OpenaiCompletions => "openai-completions",
        }
    }

    /// The dialect's documented level enum — the fallback list when
    /// no true vocabulary is known (PROVIDERS.md). The
    /// anthropic-messages dialect documents no `minimal`.
    pub fn standard_levels(self) -> &'static [&'static str] {
        match self {
            Self::AnthropicMessages => &["off", "low", "medium", "high", "xhigh", "max"],
            _ => &["off", "minimal", "low", "medium", "high", "xhigh", "max"],
        }
    }
}

/// The npm-package dialect inference — the fallback when the overlay
/// rules no dialect (the approved order: the overlay's ruling wins,
/// inference fills the gap).
pub fn infer_api(npm: Option<&str>) -> Option<Api> {
    match npm {
        Some("@ai-sdk/anthropic") => Some(Api::AnthropicMessages),
        Some("@ai-sdk/openai") => Some(Api::OpenaiResponses),
        Some("@ai-sdk/openai-compatible") => Some(Api::OpenaiCompletions),
        _ => None,
    }
}

/// Map a vendor level name onto the canonical vocabulary (tabit's
/// `ThinkingLevel` names). models.dev spells "off" as "none" (pi's
/// mapping convention) and as `null` (the `reasoning_effort: null`
/// convention); canonical names pass through; anything else has no
/// honest mapping and is dropped (reported).
fn canonical_level(name: Option<&str>) -> Option<&'static str> {
    match name {
        None | Some("none") | Some("off") => Some("off"),
        Some("minimal") => Some("minimal"),
        Some("low") => Some("low"),
        Some("medium") => Some("medium"),
        Some("high") => Some("high"),
        Some("xhigh") => Some("xhigh"),
        Some("max") => Some("max"),
        Some(_) => None,
    }
}

/// Everything the generate run observed — the curation instrument.
#[derive(Debug, Default)]
pub struct Report {
    /// Shipped providers and their model counts, in catalog order.
    pub shipped: Vec<Shipped>,
    /// Dropped models with the reason for each.
    pub dropped: Vec<Dropped>,
    /// Vendor level names that had no honest canonical mapping.
    pub level_drops: Vec<LevelDrop>,
}

#[derive(Debug)]
pub struct Shipped {
    pub provider: String,
    pub models: usize,
}

#[derive(Debug)]
pub struct Dropped {
    pub provider: String,
    pub model: String,
    pub reason: &'static str,
}

#[derive(Debug)]
pub struct LevelDrop {
    pub provider: String,
    pub model: String,
    pub level: String,
}

/// Map the source through the overlay into the emit fragment. The
/// `Err` carries every allowlisted provider that could not ship
/// (absent upstream, unresolvable ruling, zero surviving models)
/// plus any env-key collision — the sync invariant failing loudly,
/// never partially.
///
/// A shared `api_key_env` across shipped providers is a failure by
/// design (owner ruling): one set key would light up every
/// sibling's models, and half of those calls error — the plan
/// entries of one vendor are separate subscriptions on separate
/// endpoints. Colliding entries get distinct env names via the
/// overlay (derived from their models.dev ids).
pub fn map(
    api: &source::Api,
    overlay: &overlay::Overlay,
) -> Result<(emit::Fragment, Report), String> {
    let mut providers = std::collections::BTreeMap::new();
    let mut env_owners: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut report = Report::default();
    let mut failures: Vec<String> = Vec::new();

    for (id, ruling) in &overlay.providers {
        let Some(source_provider) = api.0.get(id) else {
            failures.push(format!(
                "provider `{id}` is allowlisted but absent from the source"
            ));
            continue;
        };
        match build_provider(id, source_provider, ruling, &mut report) {
            Ok(provider) => {
                report.shipped.push(Shipped {
                    provider: id.clone(),
                    models: provider.models.len(),
                });
                env_owners
                    .entry(provider.api_key_env.clone())
                    .or_default()
                    .push(id.clone());
                providers.insert(id.clone(), provider);
            }
            Err(reason) => failures.push(format!("provider `{id}`: {reason}")),
        }
    }
    for (env, owners) in &env_owners {
        if owners.len() > 1 {
            failures.push(format!(
                "api_key_env `{env}` is shared by {} — one key must not light up several providers' models; rule the env names apart in the overlay",
                owners
                    .iter()
                    .map(|owner| format!("`{owner}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    if !failures.is_empty() {
        return Err(failures.join("\n"));
    }
    Ok((emit::Fragment { providers }, report))
}

fn build_provider(
    id: &str,
    source_provider: &source::Provider,
    ruling: &overlay::Provider,
    report: &mut Report,
) -> Result<emit::Provider, String> {
    let api = match ruling.api.as_deref() {
        Some(spelling) => {
            Api::parse(spelling).ok_or_else(|| format!("unknown dialect `{spelling}`"))?
        }
        None => infer_api(source_provider.npm.as_deref())
            .ok_or("no dialect ruling and the npm package implies none")?,
    };
    let base_url = ruling
        .base_url
        .clone()
        .or_else(|| source_provider.api.clone())
        .ok_or("no base_url ruling and the source carries none")?;
    let api_key_env = ruling
        .api_key_env
        .clone()
        .or_else(|| {
            source_provider
                .env
                .as_ref()
                .and_then(|env| env.first().cloned())
        })
        .ok_or("no api_key_env ruling and the source carries no env")?;
    let name = ruling
        .name
        .clone()
        .or_else(|| source_provider.name.clone())
        .unwrap_or_else(|| id.to_string());

    let mut models = Vec::new();
    let mut drop_reasons: Vec<String> = Vec::new();
    for (model_id, source_model) in &source_provider.models {
        match build_model(id, model_id, source_model, api, report) {
            Ok(model) => models.push(model),
            Err(reason) => {
                drop_reasons.push(format!("`{model_id}`: {reason}"));
                report.dropped.push(Dropped {
                    provider: id.to_string(),
                    model: model_id.clone(),
                    reason,
                });
            }
        }
    }
    if models.is_empty() {
        let detail = if drop_reasons.is_empty() {
            "the source lists none".to_string()
        } else {
            drop_reasons.join(", ")
        };
        return Err(format!(
            "zero models survived filtering; every drop: {detail}"
        ));
    }
    Ok(emit::Provider {
        name,
        base_url,
        api: api.as_str().to_string(),
        api_key_env,
        models,
    })
}

/// Filter and map one model. `Err(reason)` is a drop, not a build
/// failure — the provider-level zero-model check above decides the
/// invariant.
fn build_model(
    provider_id: &str,
    id: &str,
    source_model: &source::Model,
    api: Api,
    report: &mut Report,
) -> Result<emit::Model, &'static str> {
    if source_model.tool_call != Some(true) {
        return Err("no tool calling");
    }
    if source_model.reasoning != Some(true) {
        return Err("not a reasoning model");
    }
    if matches!(source_model.status.as_deref(), Some("deprecated" | "alpha")) {
        return Err("deprecated or pre-release");
    }
    let inputs = source_model
        .modalities
        .as_ref()
        .and_then(|m| m.input.as_deref());
    if inputs.is_some_and(|inputs| !inputs.iter().any(|modality| modality == "text")) {
        return Err("no text input");
    }

    // `input` defaults to text-only in the consumer; emit the field
    // only when the model goes beyond that.
    let input = inputs
        .is_some_and(|inputs| inputs.iter().any(|modality| modality == "image"))
        .then(|| vec!["text", "image"]);
    let cost = source_model.cost.as_ref();
    let limit = source_model.limit.as_ref();

    let thinking_levels = thinking_levels(provider_id, id, source_model, api, report);
    Ok(emit::Model {
        id: id.to_string(),
        name: source_model.name.clone().filter(|name| name != id),
        reasoning: true,
        input,
        context_window: limit.and_then(|limit| limit.context),
        max_tokens: limit.and_then(|limit| limit.output),
        cost: emit::Cost {
            // All four legs are required by the consumer; absent
            // upstream legs are "unknown" — 0.0 per the consumer's
            // own convention.
            input: cost.and_then(|cost| cost.input).unwrap_or(0.0),
            output: cost.and_then(|cost| cost.output).unwrap_or(0.0),
            cache_read: cost.and_then(|cost| cost.cache_read).unwrap_or(0.0),
            cache_write: cost.and_then(|cost| cost.cache_write).unwrap_or(0.0),
        },
        thinking_levels,
    })
}

/// Resolve the model's thinking levels: the provider's true
/// vocabulary when the source carries one (PROVIDERS.md's fidelity
/// resolution), else the dialect's documented enum.
///
/// A vendor name maps onto the canonical vocabulary only when the
/// dialect documents the level (a "minimal" on an
/// anthropic-messages provider would be a loud request-build error
/// downstream — never advertised).
fn thinking_levels(
    provider_id: &str,
    model_id: &str,
    source_model: &source::Model,
    api: Api,
    report: &mut Report,
) -> Vec<String> {
    let Some(vocabulary) = source_model
        .reasoning_options
        .as_deref()
        .and_then(|options| {
            options
                .iter()
                .find(|option| option.kind == "effort")
                .and_then(|option| option.values.as_deref())
        })
    else {
        return api
            .standard_levels()
            .iter()
            .map(|level| level.to_string())
            .collect();
    };

    let documented = api.standard_levels();
    let mut levels: Vec<String> = Vec::new();
    for name in vocabulary {
        match canonical_level(name.as_deref()).filter(|level| documented.contains(level)) {
            Some(level) => {
                if !levels.iter().any(|existing| existing == level) {
                    levels.push(level.to_string());
                }
            }
            // Unreachable for `null` names (they map to "off", which
            // every dialect documents) — a named level without an
            // honest mapping is the drop case.
            None => report.level_drops.push(LevelDrop {
                provider: provider_id.to_string(),
                model: model_id.to_string(),
                level: name.clone().unwrap_or_default(),
            }),
        }
    }
    if levels.is_empty() {
        // A vocabulary existed but nothing mapped: fall back to the
        // documented enum rather than shipping an unusable model.
        return documented.iter().map(|level| level.to_string()).collect();
    }
    levels
}
