# ADR 0002: Keep `reqwest` for async HTTP (do not replace with `ureq`)

## Status

Accepted

## Context

`pawbun-toolkit` uses `reqwest` (version 0.12) under the `http` feature for:

1. `WebFetchTool` — async HTTP fetching (`AsyncTool` trait)
2. `WebSearchTool` — async search API calls (`AsyncTool` trait)

`pawbun-files` uses `reqwest::blocking` under the `url-source` feature for:

1. `DefaultFileLoader::load_url_sync()` — synchronous HTTP download

`ureq` was proposed as a lighter sync-only HTTP client alternative.

## Evaluation

| Crate | Sync | Async | Size | Notes |
|---|---|---|---|---|
| `reqwest` | ✅ (`blocking`) | ✅ | ~500KB | Full-featured, maintained by hyper team |
| `ureq` | ✅ | ❌ No | ~100KB | Lightweight, but no async support |

### Async scenario (`pawbun-toolkit`)

`WebFetchTool` and `WebSearchTool` implement `AsyncTool`. `ureq` has no async support. Replacing `reqwest` would require:
- Blocking HTTP inside async (bad practice)
- Or removing async tools entirely (breaking change)

### Sync scenario (`pawbun-files`)

`load_url_sync` uses `reqwest::blocking`. `ureq` could theoretically replace this, but:
- `reqwest::blocking` is already a minimal sync wrapper over the same async core
- Switching to `ureq` saves ~100KB binary size but introduces a second HTTP stack
- `ureq`'s API differs significantly; migration has non-zero risk

## Decision

**Keep `reqwest` for all HTTP scenarios.** Do not introduce `ureq`.

## Consequences

- **Positive**: Single HTTP stack across workspace; async tools remain first-class.
- **Negative**: `pawbun-files` with `url-source` feature pulls in full `reqwest` + `tokio` stack even for sync-only consumers. Mitigation: `url-source` is opt-in.

## Future reconsideration

If a lightweight sync-only HTTP client becomes a hard requirement (e.g. for `wasm32` target), evaluate `ureq` or `attohttpc` at that time.
