//! The CLI. `tabit-catalog generate` builds the catalog locally;
//! the publishing flow (CI, mirrors) wraps this command.

use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::{Parser, Subcommand};
use tabit_catalog::{generate, map, overlay};

/// The cap on a downloaded source payload — a hardening rule from
/// codex's externally-controlled catalog handling.
const SOURCE_CAP: usize = 25 * 1024 * 1024;

#[derive(Parser)]
#[command(
    name = "tabit-catalog",
    about = "Generate the tabit model catalog (PROVIDERS.md): models.dev + a curated overlay -> catalog + latest.toml"
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build the catalog from a models.dev payload and the overlay.
    Generate {
        /// The models.dev api.json source: a URL or a local path.
        #[arg(long, default_value = "https://models.dev/api.json")]
        source: String,
        /// The overlay file (the curated allowlist; variant lists
        /// live beside the default one).
        #[arg(long, default_value = "overlays/default.toml")]
        overlay: PathBuf,
        /// The output directory.
        #[arg(long, default_value = "dist")]
        out: PathBuf,
    },
}

fn main() {
    let Args { command } = Args::parse();
    let Command::Generate {
        source,
        overlay,
        out,
    } = command;
    if let Err(err) = run(&source, &overlay, &out) {
        eprintln!("tabit-catalog: {err}");
        std::process::exit(1);
    }
}

fn run(source: &str, overlay_path: &Path, out: &Path) -> Result<(), String> {
    let bytes = fetch(source)?;
    let overlay = overlay::load(overlay_path)?;
    let generated = generate(&bytes, &overlay).map_err(|err| err.to_string())?;

    print_report(&generated.report);
    std::fs::create_dir_all(out).map_err(|err| format!("{}: {err}", out.display()))?;
    let artifact_path = out.join(format!("catalog-{}.toml", generated.version));
    std::fs::write(&artifact_path, &generated.artifact)
        .map_err(|err| format!("{}: {err}", artifact_path.display()))?;
    let latest_path = out.join("latest.toml");
    std::fs::write(&latest_path, &generated.latest)
        .map_err(|err| format!("{}: {err}", latest_path.display()))?;
    println!(
        "wrote {} ({} bytes) and {} — version {}, sha256 {}",
        artifact_path.display(),
        generated.artifact.len(),
        latest_path.display(),
        generated.version,
        generated.sha256,
    );
    Ok(())
}

/// Load the source payload: over HTTP (timeout + hard size cap) or
/// from a local path (offline builds, tests, reproducibility).
fn fetch(source: &str) -> Result<Vec<u8>, String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|err| format!("http client: {err}"))?;
        let response = client
            .get(source)
            .send()
            .and_then(|response| response.error_for_status())
            .map_err(|err| format!("GET {source}: {err}"))?;
        if response
            .content_length()
            .is_some_and(|length| length as usize > SOURCE_CAP)
        {
            return Err(format!(
                "GET {source}: payload exceeds {} bytes",
                SOURCE_CAP
            ));
        }
        let bytes = response
            .bytes()
            .map_err(|err| format!("GET {source}: {err}"))?;
        if bytes.len() > SOURCE_CAP {
            return Err(format!(
                "GET {source}: payload exceeds {} bytes",
                SOURCE_CAP
            ));
        }
        Ok(bytes.to_vec())
    } else {
        let bytes = std::fs::read(source).map_err(|err| format!("{source}: {err}"))?;
        if bytes.len() > SOURCE_CAP {
            return Err(format!("{source}: payload exceeds {} bytes", SOURCE_CAP));
        }
        Ok(bytes)
    }
}

/// The curation observation instrument: everything that shipped,
/// dropped, or lost a level mapping.
fn print_report(report: &map::Report) {
    for shipped in &report.shipped {
        println!("shipped  {}: {} models", shipped.provider, shipped.models);
    }
    for dropped in &report.dropped {
        println!(
            "dropped  {}/{}: {}",
            dropped.provider, dropped.model, dropped.reason
        );
    }
    for drop in &report.level_drops {
        println!(
            "level    {}/{}: `{}` has no canonical mapping — dropped",
            drop.provider, drop.model, drop.level
        );
    }
}
