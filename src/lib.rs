//! The tabit catalog generator (PROVIDERS.md, the generator
//! thread): builds the downloadable model catalog from models.dev
//! plus a curated provider overlay, and stamps it for the client
//! channel.
//!
//! Pipeline: parse source → filter by the overlay allowlist → map
//! to tabit's providers-fragment shape → emit deterministic TOML →
//! validate through the real consumer schema (`tabit-config`, the
//! crates.io release) → stamp version + sha256 into `latest.toml`.
//!
//! Invariants (a violation fails the build loudly):
//! - every overlay-allowlisted provider ships at least one model
//!   (the sync invariant, PROVIDERS.md);
//! - the emitted artifact parses as a `tabit-config` fragment;
//! - output is byte-deterministic for identical input.

pub mod emit;
pub mod map;
pub mod overlay;
pub mod source;
pub mod stamp;

use thiserror::Error;

/// The artifact schema major version the channel speaks — the
/// consumer's `min_schema` gate (PROVIDERS.md). Bump only on an
/// incompatible artifact-schema change, together with the consumer.
pub const MIN_SCHEMA: u32 = 1;

/// Everything one `generate` run produced.
#[derive(Debug)]
pub struct Generated {
    /// The artifact body: a tabit providers fragment — exactly the
    /// bytes the mirrors serve and the client checksums.
    pub artifact: String,
    /// The `latest.toml` pointer body.
    pub latest: String,
    /// The version string (also the artifact filename suffix).
    pub version: String,
    /// The artifact's sha256, lowercase hex.
    pub sha256: String,
    /// What happened along the way (the curation observation
    /// instrument; the CLI prints it).
    pub report: map::Report,
}

#[derive(Debug, Error)]
pub enum GenerateError {
    /// The models.dev payload does not parse.
    #[error("source does not parse: {0}")]
    Source(String),
    /// The overlay file is unreadable or malformed.
    #[error("overlay: {0}")]
    Overlay(String),
    /// The sync invariant failed: an allowlisted provider ships no
    /// models, or its ruling cannot be resolved (reasons attached).
    /// The build must not succeed in this state.
    #[error("sync invariant failed:\n{0}")]
    Sync(String),
    /// The emitted artifact failed consumer-schema validation —
    /// generator/consumer schema drift, an internal failure.
    #[error("artifact rejected by the consumer schema: {0}")]
    Validation(String),
}

/// Run the pipeline over a raw models.dev `api.json` payload.
pub fn generate(
    source_json: &[u8],
    overlay: &overlay::Overlay,
) -> Result<Generated, GenerateError> {
    let api = source::parse(source_json).map_err(GenerateError::Source)?;
    let (fragment, report) = map::map(&api, overlay).map_err(GenerateError::Sync)?;
    let artifact = emit::emit(&fragment).map_err(GenerateError::Validation)?;
    // The consumer itself is the schema authority: the artifact must
    // parse through tabit-config before it is stamped for publishing.
    tabit_config::TabitConfig::from_toml_str(&artifact, std::path::Path::new("catalog.toml"))
        .map_err(|err| GenerateError::Validation(err.to_string()))?;
    let (version, sha256) = stamp::stamp(&artifact);
    let latest = stamp::latest_toml(&version, &sha256);
    Ok(Generated {
        artifact,
        latest,
        version,
        sha256,
        report,
    })
}
