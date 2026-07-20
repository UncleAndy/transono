# Design: Public API Rustdoc for `transono`

**Date:** 2026-07-20  
**Status:** Approved (pending implementation plan)  
**Language of docs:** English

---

## Problem

The crate exposes a substantial public API (`core`, `line`, `providers`, `audio`, `runtime`, `ctl`, `console`, `testing`), but rustdoc coverage is almost absent. A few existing `//!` / `///` comments are in Russian or incomplete. Consumers cannot rely on `cargo doc` to understand how to wire providers, lines, and audio pipelines.

## Goals

1. Document **all public (`pub`) library items** with idiomatic English rustdoc.
2. Deliver documentation **in layers** so `core` / `line` / `providers` become usable first.
3. Enable `#![warn(missing_docs)]` on the crate to keep coverage honest.
4. Use existing Russian architecture docs as **semantic source**, not as verbatim translation targets for rustdoc.

## Non-goals

- Documenting `pub(crate)` or private items.
- Full rustdoc for binary crates (`transono`, `transonovirt`) beyond an optional short `//!` on `main`.
- Rewriting or translating markdown under `docs/architecture/` as part of this work.
- Behavioral refactors “while documenting”.
- Enabling `#![deny(missing_docs)]` in this effort.

## Approach

**Layer-by-layer public API documentation** (chosen over “face-first stubs” or “generate TODO scaffolds”).

Each phase adds module-level `//!`, type/trait docs, and method docs for that layer’s public surface. Russian inline comments in those modules are replaced with English rustdoc that preserves meaning.

---

## Conventions

### Depth (standard rustdoc)

- Most items: 1–3 sentences covering role, purpose, and non-obvious constraints (threading, ownership, latency / no-alloc on hot paths).
- Fallible APIs: `# Errors` when error cases matter to callers.
- Panic paths: `# Panics` only when the API can panic.
- Entry points: `# Examples` for `TranslationLine`, `Provider` / OpenAI config, and representative pipeline/processor usage.
  - Prefer compiling doctests when feasible.
  - Use `no_run` or `ignore` when hardware, network, or secrets are required; mark why briefly.

### Module docs (`//!`)

- One-line role summary.
- Short context: place in the stack, what the module owns, what it deliberately does **not** own.
- Optional `See also` pointing at related modules or repo docs paths (e.g. `docs/architecture/01-runtime.md`).

### Style

- Describe contracts and intent; do not restate obvious signatures.
- No `/// TODO` placeholders left in tree.
- Do not paste long ADR text into rustdoc; summarize and link if needed.

### Semantic sources

- `docs/architecture/00-philosophy.md`
- `docs/architecture/01-runtime.md`
- `docs/architecture/02-translation-bridge.md`
- `docs/realtime_api.md`
- `docs/decisions.md`
- Existing Russian `//!` / `///` in `src/` (translate sense into English)

---

## Phases

### Phase 1 — Core public API

| Area | Document |
|------|----------|
| `src/lib.rs` | Crate-level overview: purpose, layer diagram in prose, module map |
| `core` | `CoreError` / `Result`, `Provider` / `ProviderSession`, transport + WebSocket, protocol, session / session events, provider commands / events |
| `line` | `TranslationLine`, `LineState`, lifecycle and processor configuration |
| `providers` | OpenAI `realtime` and `translation` public configs, providers, sessions, protocol types |

**Exit criteria:** `cargo doc --no-deps` shows a coherent path from crate root → provider → `TranslationLine`. `#![warn(missing_docs)]` is enabled; remaining warnings are expected for Phase 2 modules.

### Phase 2 — Audio, runtime, control

| Area | Document |
|------|----------|
| `audio` | Devices, input/output traits, pipeline / processors / encoders, buffers and pools, CPAL and PipeWire backends; replace remaining Russian inline docs |
| `runtime` | Ports, mixer, link/splitter; note draft / unstable status where accurate |
| `ctl` | `Backend`, device sets/status, command entry points |
| `console`, `testing` | Short module purpose docs (public surface only) |

**Exit criteria:** All library `pub` items documented; `cargo doc --no-deps` succeeds; Russian inline rustdoc removed or replaced; `missing_docs` warnings for documented modules cleared.

---

## Lint policy

```rust
// src/lib.rs
#![warn(missing_docs)]
```

- Introduced at the start of Phase 1.
- Warnings for not-yet-documented Phase 2 items are acceptable until Phase 2 completes.
- Do **not** promote to `deny` in this design.

---

## Content templates

### Trait / struct

- Role in the system.
- Important invariants (e.g. `Send + Sync`, audio-thread rules, ownership of buffers).
- Relationship to neighboring types (`Provider` ↔ `ProviderSession` ↔ `TranslationLine`).

### Method

- Effect and return value in caller terms.
- `# Errors` / `# Panics` when applicable.
- Hot-path notes only when allocation or copies are constrained.

### Example sketch (conceptual)

```rust
/// Runs one translation line from capture to playback via a provider.
///
/// # Examples
///
/// ```no_run
/// // Requires audio devices and provider credentials.
/// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
/// # Ok(())
/// # }
/// ```
```

---

## Verification

After each phase:

1. `cargo doc --no-deps`
2. Spot-check crate root, `core`, `line`, and OpenAI provider pages (Phase 1); audio/ctl pages (Phase 2)
3. Confirm no Russian rustdoc remains in touched modules
4. Ensure documentation-only changes do not introduce clippy regressions

---

## Definition of done

- [ ] Every `pub` library item has English rustdoc
- [ ] Crate and major modules have `//!` overviews
- [ ] Entry points have Errors/Panics/Examples as specified
- [ ] `#![warn(missing_docs)]` enabled; Phase 2 complete ⇒ no outstanding missing-docs for public API
- [ ] Existing Russian inline docs converted or replaced
- [ ] No intentional code behavior changes

---

## Out of order / risks

- **Unstable runtime API:** document current behavior and mark draft status rather than inventing future guarantees.
- **Doctests vs hardware:** prefer `no_run` over brittle CI failures.
- **Large `audio` surface:** Phase 2 may be split into commits by subdirectory (`device`/`pipeline`, `processors`, `cpal`/`pipewire`) without changing this design.
