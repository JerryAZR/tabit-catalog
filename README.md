# tabit-catalog

The tabit model catalog **generator and channel** (PROVIDERS.md's
generator thread): builds the catalog from models.dev plus a
curated overlay, publishes it as immutable versioned artifacts, and
tabit downloads it as `providers.auto.toml` — the zero-config
provider story: set one key, pick a model.

- Consumers: the **Using the catalog in tabit** section below.
- Curators: everything else (generator, overlay, design, CI).

## Using the catalog in tabit

The catalog ships 18 providers — the common ones (Anthropic,
OpenAI, Google, DeepSeek, Fireworks, Mistral, xAI, Groq, Cerebras,
Together) plus the coding/token plans of the CN vendors in both CN
and global variants (Kimi, GLM/Zhipu, MiniMax, Alibaba/Qwen).
Every entry is a reasoning, tool-calling model with costs, context
windows, and true thinking-level vocabularies.

**How it reaches your machine.** After boot, tabit refreshes the
catalog in the background (never blocking startup) when its cache
is older than 24h: fetch `latest.toml` from the first mirror that
answers, download the immutable `catalog-<version>.toml`, verify
the sha256, write `providers.auto.toml` next to your config. Any
failure keeps the last cache with a warning — the channel is
read-only and failure-soft. `catalog.auto_update = false` disables
it outright.

**Enabling.** Set the mirror list once in `~/.tabit/settings.toml`
(until tabit ships this repo's coordinates as its built-in
default):

```toml
[catalog]
urls = [
    "https://cdn.jsdelivr.net/gh/JerryAZR/tabit-catalog@catalog", # China-reliable
    "https://raw.githubusercontent.com/JerryAZR/tabit-catalog/catalog",
]
# ttl_hours = 24   # default
# auto_update = true
```

**Variant lists.** The channel publishes several lists, each an
independent catalog in its own subdirectory — point `catalog.urls`
at whichever fits (the client concatenates `{base}/latest.toml`,
so a subdirectory is just another base):

| list | jsDelivr base (add `/cn`, `/global`, … before use) | carries |
|---|---|---|
| default | `https://cdn.jsdelivr.net/gh/JerryAZR/tabit-catalog@catalog` (root — the built-in coordinates) | the union of everything curated |
| `cn` | `https://cdn.jsdelivr.net/gh/JerryAZR/tabit-catalog@catalog/cn` | the CN plans + providers reachable from China without a proxy |
| `global` | `https://cdn.jsdelivr.net/gh/JerryAZR/tabit-catalog@catalog/global` | the global plans + western providers |
| `openrouter` | `https://cdn.jsdelivr.net/gh/JerryAZR/tabit-catalog@catalog/openrouter` | the aggregator as an opt-in — its breadth where it was noise in the default list |

(The raw mirror is the same layout:
`https://raw.githubusercontent.com/JerryAZR/tabit-catalog/catalog/<list>`.)

```toml
[catalog]
urls = ["https://cdn.jsdelivr.net/gh/JerryAZR/tabit-catalog@catalog/cn"]
# add the raw base as a second entry if you want the fallback mirror
```

**Keys.** One environment variable per provider — by design, so a
set key lights up exactly one provider's models (never a
half-erroring list). In the default list:

| plan | env var | endpoint |
|---|---|---|
| Kimi Coding (CN / global) | `KIMI_CODE_PLAN_CN_API_KEY` / `KIMI_CODE_PLAN_GLOBAL_API_KEY` | `api.kimi.com/coding` / `api.kimi.ai/coding` |
| GLM Coding (CN / global) | `ZHIPU_CODING_PLAN_CN_API_KEY` / `ZAI_CODING_PLAN_GLOBAL_API_KEY` | `open.bigmodel.cn/api/anthropic` / `api.z.ai/api/anthropic` |
| MiniMax Token (CN / global) | `MINIMAX_CN_CODING_PLAN_API_KEY` / `MINIMAX_CODING_PLAN_API_KEY` | `api.minimax.cn` / `api.minimax.io` |
| Alibaba Token (CN / global) | `ALIBABA_TOKEN_PLAN_CN_API_KEY` / `ALIBABA_TOKEN_PLAN_API_KEY` | `token-plan.cn-beijing` / `token-plan.ap-southeast-1` |
| payg providers | the vendor's standard name (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `DEEPSEEK_API_KEY`, …) | |

(`~/.tabit/auth.toml` entries keyed by provider id win over the env
var, as with any tabit provider.)

**Model references** are `<provider>/<model>`, e.g.
`kimi-code-plan-cn/kimi-for-coding` or
`zhipuai-coding-plan/glm-5.3`.

**Overriding.** Your `providers.toml` always wins: a provider you
declare keeps its fields and its model ids win whole — the catalog
only fills in models you haven't declared. Delete your entry to
get the catalog's back. A provider the catalog doesn't know (a
local server, a niche vendor) is declared in `providers.toml`
exactly as before — the catalog never *creates* support, it only
carries data.

## How the generator works

```
tabit-catalog generate [--source <url|path>] [--overlay <file>] [--out <dir>]
```

- `--source` defaults to `https://models.dev/api.json`; a local
  path builds fully offline (tests, reproducibility). Downloads are
  timeout- and size-capped (25 MiB).
- `--overlay` defaults to `overlays/default.toml` — the curated
  allowlist (variant lists beside it).
- `--out` defaults to `dist/`: `catalog-<version>.toml` (an
  ordinary tabit providers fragment), `latest.toml` (the pointer
  `{version, sha256, min_schema}`), and a stdout report — every
  shipped provider, every dropped model with its reason, every
  unmapped thinking-level name.

Pipeline: parse source → overlay allowlist → model filters → map
→ emit deterministic TOML → **validate through the real consumer
schema** (`tabit-config`, the crates.io release) → stamp.

Invariants (violations fail the build): every allowlisted provider
ships at least one model (the sync invariant); unique model ids;
unique `api_key_env`; the artifact parses as a tabit fragment;
byte-deterministic output.

## Design choices

**models.dev as the source** — the ecosystem's shared raw
material: pi, opencode, kimi-code, MiMo-Code and others all
consume it, so it stays fresh and field-checked. It carries costs,
limits, modalities, and `reasoning_options` — the provider's
*actual* effort vocabulary. Derived data is MIT; attribution to
models.dev belongs in the artifact header when tabit's installer
prepends it. What models.dev cannot supply — which tabit dialect a
provider speaks, and endpoints in tabit's URL convention — is the
overlay's job: curation data, never code (Rule 1's spirit).

**A curated allowlist, not everything** — the generic agents ship
all 100–200 upstream providers and drown the picker. This catalog
covers the providers ~80% of users use; niche vendors are user
config (`providers.toml` is the seam). Aggregators (openrouter &
co.) are excluded: a meta-provider re-introduces the noise.

**anthropic-messages preferred** — the claude-code surface is the
first compat surface every serious provider maintains; compat is
the provider's server-side problem, ours is a faithful engine
(PROVIDERS.md). OpenAI-only vendors use their own dialect.

**Coding/token plans over pay-as-you-go** — plans are the common
usage of the CN vendors; both CN and global variants ship. Plan
models carry no per-token costs upstream (subscription "included"
pricing) and ship as `0.0` legs.

**No env-key sharing, enforced** — models.dev gives sibling
entries (CN/global, plan/payg) the same env var; if the catalog
repeated that, one set key would light up every sibling's models
and half the calls would error against the wrong endpoint. The
generator fails the build on any shared `api_key_env`; colliding
families get names derived from their models.dev ids.

**True thinking-level vocabularies** — PROVIDERS.md's open
question, resolved: `reasoning_options` values map onto the
canonical levels (`none`/`null` → `off`; unmappable names dropped
and reported), filtered to what the dialect documents (no
`minimal` under anthropic-messages — a level the engine would
reject at request build is never advertised). No vocabulary
upstream → the dialect's documented enum.

**Content-unique versions** — `version = <utc-date>-<sha256-prefix>`:
same version implies identical bytes (ZCode's revision-monotonicity
lesson). Artifacts are immutable and accumulate; a run whose
content matches the published pointer is a CI no-op.

**Never shipped**: engine quirks (the vocabulary stays empty until
a deviation is *observed* in our own traffic), `extra_body`
(deleted from tabit by design), anything not parseable as a plain
providers fragment.

## The overlays

`overlays/*.toml` are data, not code — the editorial lines. Every
list (default, cn, global, openrouter) is the same mechanism: a
plain overlay file and a `generate` run — no projection, no tags,
no per-list generator features; a future split (plan/payg, a
single vendor, anything) is just another file. Presence in
`[providers.*]` is the allowlist; entries rule the dialect (`api`),
the endpoint (`base_url`, tabit convention), and optionally
`api_key_env` / display `name`. Endpoint rulings are verified
against vendor configs where possible (ZCode's builtin catalog,
pi's provider data) and carry provenance notes.

Adding a provider: add an `[providers.<id>]` block (verify the
endpoint), run `generate`, review the report.

## Publication

`.github/workflows/publish.yml` wraps `generate`: on push to main
(overlay edits publish immediately), daily at 03:17 UTC, and
manually — generating every list (default at the root, variants
in subdirectories). The channel is the dedicated **`catalog`
branch**: `latest.toml` per list (the mutable pointer) plus the
immutable `catalog-<version>.toml` files, which accumulate
forever (they are cache-forever URLs; a version must never change
meaning). A run where no list changed commits nothing, and a
concurrency group keeps runs from racing.

The mirror bases (tabit's built-in `catalog.urls` default):

- `https://cdn.jsdelivr.net/gh/JerryAZR/tabit-catalog@catalog`
  (jsDelivr — reliable from China; ~12h branch-ref cache on the
  pointer, acceptable under the client's 24h TTL; the immutable
  artifact URLs sidestep CDN staleness entirely)
- `https://raw.githubusercontent.com/JerryAZR/tabit-catalog/catalog`
  (GitHub raw)

Attribution: the catalog data is derived from
[models.dev](https://models.dev) (MIT).
