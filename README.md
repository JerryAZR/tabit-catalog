# tabit-catalog

The tabit model catalog **generator** (PROVIDERS.md's generator
thread): builds the downloaded catalog artifact locally. The
publishing flow (CI, mirrors) wraps this command.

```
tabit-catalog generate [--source <url|path>] [--overlay <file>] [--out <dir>]
```

- `--source` defaults to `https://models.dev/api.json`; a local path
  builds fully offline (tests, reproducibility). Downloads are
  timeout- and size-capped (25 MiB).
- `--overlay` defaults to `overlay.toml` — the curated allowlist
  (see below).
- `--out` defaults to `dist/`, receiving:
  - `catalog-<version>.toml` — the artifact, an ordinary tabit
    providers fragment;
  - `latest.toml` — the pointer `{version, sha256, min_schema}`;
  - a report on stdout: every shipped provider, every dropped
    model with its reason, every unmapped thinking-level name.

## How it works

models.dev's `api.json` supplies the models (costs, context
windows, modalities, reasoning, the *true* level vocabulary via
`reasoning_options`). The overlay supplies the editorial rulings.
The pipeline filters, maps, emits deterministic TOML, then
**validates the artifact through the real consumer schema**
(`tabit-config`, the crates.io release — the round-trip gate that
makes generator/consumer drift a build failure), and stamps
`version = <utc-date>-<sha256-prefix>` (content-unique: same
version implies identical bytes).

Filters (the 80% ruling, 2026-10): the overlay's allowlist;
per-model `tool_call`, `reasoning`, `status ∉ {deprecated, alpha}`,
text input. Dialects prefer `anthropic-messages` (the claude-code
surface providers maintain first). Coding/token plans ship in
preference to pay-as-you-go entries (both CN and global variants
when they exist); a shared `api_key_env` across shipped providers
fails the build — one set key must never light up several
providers' models. Endpoint rulings follow tabit's URL
conventions (engines append their request paths).

Invariants (violations fail the build): an allowlisted provider
must ship at least one model (the sync invariant); unique model
ids; unique `api_key_env` per provider; byte-deterministic output.

## The overlay

`overlay.toml` is data, not code — the catalog's editorial line.
Presence in `[providers.*]` is the allowlist; entries rule the
dialect (`api`), the endpoint (`base_url`, tabit convention), and
optionally `api_key_env` / display `name`. Niche providers stay
user config (`providers.toml`); aggregators (openrouter & co.) are
deliberately excluded (owner ruling).

Adding a provider: add an `[providers.<id>]` block (verify the
endpoint), run `generate`, review the report.
