# ADR 0001: Keep `image` crate (do not replace with `image-meta`)

## Status

Accepted

## Context

`pawbun-files` uses the `image` crate (version 0.25) under the `image-meta` feature for:

1. **Image dimension extraction** in `DefaultFileLoader::extract_image_dimensions()` (`loader.rs`)
2. **Image downgrading** in `downgrade_image()` (`constraints.rs`) — full decode, resize with Lanczos3, and re-encode
3. **Test helpers** — creating in-memory PNG images for unit tests

The `image-meta` crate was proposed as a lighter alternative that only extracts metadata (dimensions, format) without decoding the full image.

## Evaluation

| Capability | `image` | `image-meta` | Required? |
|---|---|---|---|
| JPEG dimension extraction | ✅ | ✅ | Yes |
| PNG dimension extraction | ✅ | ✅ | Yes |
| WebP dimension extraction | ✅ | ⚠️ Partial | Yes |
| Full decode + resize + re-encode | ✅ | ❌ No | **Yes** (`downgrade_image`) |
| In-memory test image generation | ✅ | ❌ No | Yes (tests) |

`image-meta` covers only ~30% of current usage. The `downgrade_image` function is a core feature for the planned `Auto` overflow mode (resize images to fit provider constraints).

## Decision

**Keep the `image` crate.** Do not replace it with `image-meta`.

## Consequences

- **Positive**: No code changes required; `downgrade_image` continues to work.
- **Negative**: `image` crate adds ~2-3s to clean compile time and pulls in additional codec dependencies (avif, webp, etc.). This is acceptable for the `image-meta` feature, which is opt-in.

## Future reconsideration

If `downgrade_image` is removed or moved to a separate crate, re-evaluate `image-meta` for dimension extraction only.
