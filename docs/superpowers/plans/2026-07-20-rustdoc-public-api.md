# Public API Rustdoc Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add English rustdoc to every public library item in `transono`, phased as core/line/providers first, then audio/runtime/ctl, with `#![warn(missing_docs)]`.

**Architecture:** Documentation-only changes. Module `//!` overviews plus `///` on public types/traits/methods. Semantic source: `docs/architecture/*` and existing Russian inline comments (rewrite into English). No behavior changes.

**Tech Stack:** Rust rustdoc (`///`, `//!`), `cargo doc --no-deps`, `#![warn(missing_docs)]`.

**Spec:** `docs/superpowers/specs/2026-07-20-rustdoc-public-api-design.md`

**Git note:** Do not commit unless the user explicitly asks. Skip commit steps or pause for approval.

---

## File map

### Phase 1 (touch)
| File | Responsibility |
|------|----------------|
| `src/lib.rs` | Crate docs + `#![warn(missing_docs)]` |
| `src/core/mod.rs` + all `src/core/*.rs` | Core abstractions |
| `src/line/mod.rs`, `line.rs`, `state.rs` | Translation line API |
| `src/providers/mod.rs` | Providers root |
| `src/providers/openai/**` | OpenAI realtime + translation |

### Phase 2 (touch)
| File | Responsibility |
|------|----------------|
| `src/audio/**` | Audio I/O, pipeline, DSP, backends |
| `src/runtime/**` | Ports / mixer / link (draft) |
| `src/ctl/**` | Virtual device control |
| `src/console/mod.rs`, `src/testing/**` | Short module docs |

---

## Phase 1 — Core public API

### Task 1: Crate root + missing_docs lint

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Add crate-level docs and lint**

Replace the top of `src/lib.rs` with:

```rust
//! Real-time speech translation library (TRANSONO).
//!
//! Builds streaming speech pipelines on top of AI providers. The main
//! application unit is a [`line::TranslationLine`]: capture → process →
//! provider session → playback.
//!
//! # Layers
//!
//! - [`core`] — transport, protocol, and provider abstractions
//! - [`providers`] — concrete AI backends (OpenAI Realtime / Translation)
//! - [`line`] — one independent translation stream
//! - [`audio`] — devices, buffers, DSP pipeline
//! - [`runtime`] — experimental audio graph helpers
//! - [`ctl`] — OS virtual audio device management
//!
//! See also: `docs/architecture/` in the repository.

#![warn(missing_docs)]

pub mod audio;
pub mod core;
pub mod providers;
pub mod testing;
pub mod runtime;
pub mod console;
pub mod ctl;
pub mod line;
```

- [ ] **Step 2: Verify docs build**

Run: `cargo doc --no-deps 2>&1 | tail -40`

Expected: succeeds; many `missing documentation` warnings for undocumented `pub` items (expected until later tasks).

- [ ] **Step 3: Commit only if user asks**

---

### Task 2: `core` module + errors

**Files:**
- Modify: `src/core/mod.rs`
- Modify: `src/core/error.rs`

- [ ] **Step 1: Module docs in `src/core/mod.rs`**

```rust
//! Provider-agnostic core: transport, protocol, session, and errors.
//!
//! High-level orchestration ([`crate::line::TranslationLine`]) depends on
//! these abstractions, not on a specific AI vendor or wire format.

pub mod error;
pub mod protocol;
pub mod provider;
pub mod provider_command;
pub mod provider_event;
pub mod transport;
pub mod websocket;
pub mod session;
pub mod session_event;
```

- [ ] **Step 2: Document `Result`, `CoreError`, `TransportError`, `ProtocolError`**

In `src/core/error.rs`, add docs such as:

```rust
//! Error types shared across core, audio, and providers.

/// Result alias used by core and line APIs.
pub type Result<T> = std::result::Result<T, CoreError>;

/// Top-level library error.
///
/// Prefer mapping lower-level failures into these variants rather than
/// panicking on audio or session paths.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Transport-layer failure (disconnect, WebSocket I/O).
    #[error(transparent)]
    Transport(#[from] TransportError),
    // ... document each variant similarly (one line each)
}

/// Errors from bidirectional byte/text transports.
#[derive(Debug, Error)]
pub enum TransportError { /* one-line docs per variant */ }

/// Protocol encode/decode or HTTP construction failures.
#[derive(Debug, Error)]
pub enum ProtocolError { /* one-line docs per variant */ }
```

Document every enum variant with a one-line `///`.

- [ ] **Step 3: Verify**

Run: `cargo doc --no-deps -p transono 2>&1 | rg "core::" | head -20`

Expected: fewer/no `missing documentation` for `core::error`.

---

### Task 3: Provider / session / transport / protocol

**Files:**
- Modify: `src/core/provider.rs`
- Modify: `src/core/provider_command.rs`
- Modify: `src/core/provider_event.rs`
- Modify: `src/core/session.rs`
- Modify: `src/core/session_event.rs`
- Modify: `src/core/protocol.rs`
- Modify: `src/core/transport.rs`
- Modify: `src/core/websocket.rs`

- [ ] **Step 1: Document `Provider` and `ProviderSession`**

In `src/core/provider.rs`:

```rust
//! AI provider factory and session spawning contracts.

use async_trait::async_trait;
// ... existing imports ...

/// Running provider session that consumes capture and drives playback.
///
/// Implementations typically own a WebSocket (or similar) connection and
/// bridge encoded audio in both directions until `cancel` fires.
pub trait ProviderSession {
    /// Spawn the session on the Tokio runtime.
    ///
    /// Returns a join handle that yields the pipelines when the session ends
    /// so the caller can reclaim DSP state after stop.
    ///
    /// # Errors
    ///
    /// The join handle resolves to [`CoreError`] if the session fails
    /// (transport, protocol, or internal processing).
    fn spawn(
        self,
        capture_stream: BoxStream<'static, Audio>,
        playback_sink: BoxSink<'static, Audio, CoreError>,
        pipelines: Pipelines,
        cancel: CancellationToken,
        event_tx: Option<mpsc::UnboundedSender<SessionEvent>>,
    ) -> JoinHandle<Result<Pipelines>>;
}

/// Factory for provider sessions and the audio format they expect.
///
/// # Examples
///
/// ```no_run
/// use transono::core::provider::Provider;
/// use transono::providers::openai::translation::{
///     OpenAITranslationConfig, OpenAITranslationProvider,
/// };
///
/// # async fn demo() -> transono::core::error::Result<()> {
/// let provider = OpenAITranslationProvider::new(OpenAITranslationConfig::from_env()?);
/// let _session = provider.create_session().await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait Provider {
    /// Session type produced by this provider.
    type Session: ProviderSession;

    /// Open a new realtime session with the remote backend.
    ///
    /// # Errors
    ///
    /// Returns transport/protocol errors if the connection or handshake fails.
    async fn create_session(&self) -> Result<Self::Session>;

    /// Encoded audio format required by the remote session.
    fn audio_format(&self) -> EncodedAudioFormat;
}
```

Adjust the example if `OpenAITranslationConfig::from_env` does not exist — use the real constructor from `config.rs` (read the file and match the API). Prefer `no_run` if credentials/env are required.

- [ ] **Step 2: Document remaining core types**

Apply the same style:

| File | Focus |
|------|--------|
| `protocol.rs` | `Protocol` trait: encode/decode commands & events |
| `transport.rs` | `Transport`, `TransportData`, `serialize_bytes_as_str` |
| `websocket.rs` | `WebSocketTransport` + public methods (`# Errors`) |
| `session.rs` | `Session` trait |
| `session_event.rs` | `SessionEvent` variants |
| `provider_command.rs` | `ProviderCommand` variants |
| `provider_event.rs` | `ProviderEvent` variants |

Semantic hints from `docs/architecture/00-philosophy.md` (transport vs provider independence).

- [ ] **Step 3: Verify**

Run: `cargo doc --no-deps 2>&1 | rg "missing documentation.*core::" || true`

Expected: empty (or only re-exports if any).

---

### Task 4: `line` module

**Files:**
- Modify: `src/line/mod.rs`
- Modify: `src/line/state.rs`
- Modify: `src/line/line.rs`

- [ ] **Step 1: Module + state docs**

`src/line/mod.rs`:

```rust
//! One independent speech-translation stream ([`TranslationLine`]).
//!
//! A line owns audio I/O, DSP pipelines, and a provider session. Lines do
//! not know about each other; multi-party coordination belongs to a higher
//! layer (TranslationBridge — see architecture docs).

pub mod line;
pub mod state;

pub use line::*;
pub use state::*;
```

`src/line/state.rs`:

```rust
//! Lifecycle states for [`super::TranslationLine`].

/// Lifecycle of a [`crate::line::TranslationLine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineState {
    /// Constructed; not yet running.
    Created,
    /// Capture, playback, and provider session are active.
    Running,
    /// Shutdown in progress (reserved / transitional).
    Stopping,
    /// Fully stopped; may be started again after re-attach of audio.
    Stopped,
}
```

- [ ] **Step 2: Document `TranslationLine` and all `pub` methods**

In `src/line/line.rs`, document the struct and each public method. Key points:

- `new` — builds pipelines, calls `auto_configure`
- `add_*` / `clear_*` / `with_*` — mutate processors only when not `Running`; `# Errors` for running state
- `run` / `stop` — session lifecycle; `# Errors` for missing audio, session failure
- `latency`, `state`, accessors — one-liners

Example shape for the struct:

```rust
/// Single capture→provider→playback translation stream.
///
/// Parameterized by a [`Provider`](crate::core::provider::Provider). Prefer
/// configuring processors before [`Self::run`].
///
/// # Examples
///
/// ```no_run
/// // Requires real audio devices and provider credentials.
/// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
/// # Ok(())
/// # }
/// ```
pub struct TranslationLine<P>
where
    P: Provider,
{
    // fields stay undocumented unless pub
}
```

For fallible methods that return `Err(CoreError::Internal(...))` when running:

```rust
/// Append a DSP stage on the capture path.
///
/// # Errors
///
/// Returns [`CoreError::Internal`] if the line is already [`LineState::Running`].
pub fn add_input_processor(&mut self, processor: Processor) -> Result<()> {
```

- [ ] **Step 3: Verify**

Run: `cargo doc --no-deps 2>&1 | rg "missing documentation.*(line::|TranslationLine)" || true`

Expected: empty.

---

### Task 5: Providers — OpenAI shared + realtime

**Files:**
- Modify: `src/providers/mod.rs`
- Modify: `src/providers/openai/mod.rs`
- Modify: `src/providers/openai/error.rs`
- Modify: `src/providers/openai/realtime/mod.rs`
- Modify: `src/providers/openai/realtime/config.rs`
- Modify: `src/providers/openai/realtime/provider.rs`
- Modify: `src/providers/openai/realtime/session.rs`
- Modify: `src/providers/openai/realtime/protocol.rs`
- Modify: `src/providers/openai/realtime/commands.rs`
- Modify: `src/providers/openai/realtime/events.rs`

- [ ] **Step 1: Module docs**

```rust
// src/providers/mod.rs
//! Concrete AI provider implementations.
//!
//! Application code should depend on [`crate::core::provider::Provider`]
//! and pick a backend from this module (today: OpenAI).

pub mod openai;
```

```rust
// src/providers/openai/mod.rs
//! OpenAI Realtime and Translation API backends.

pub mod realtime;
pub mod translation;
pub mod error;
```

```rust
// src/providers/openai/realtime/mod.rs
//! OpenAI Realtime API provider (bidirectional speech / tools).
//!
//! Wire format details live in `commands` / `events` / `protocol`.
//! Session orchestration is in [`RealtimeSession`].

pub mod commands;
// ... keep existing mod/use lines
```

- [ ] **Step 2: Document public types**

For each public struct/enum/method:

- `OpenAiError` — variants
- `OpenAIRealtimeConfig`, `TurnMode` — fields that are `pub`
- `OpenAIRealtimeProvider` — construction + `Provider` impl notes
- `RealtimeSession` — `ProviderSession` behavior
- `RealtimeProtocol`, config structs in `protocol.rs`
- `ProtocolCommand` / `ProtocolEvent` and payload structs

Use `docs/realtime_api.md` for accurate event/command names. Keep docs short; do not paste the full OpenAI schema.

Include one `# Examples` `no_run` on `OpenAIRealtimeProvider` or config builder if a clear constructor exists.

- [ ] **Step 3: Verify**

Run: `cargo doc --no-deps 2>&1 | rg "missing documentation.*providers::openai::realtime" || true`

Expected: empty.

---

### Task 6: Providers — OpenAI translation + Phase 1 gate

**Files:**
- Modify: `src/providers/openai/translation/mod.rs`
- Modify: `src/providers/openai/translation/config.rs`
- Modify: `src/providers/openai/translation/provider.rs`
- Modify: `src/providers/openai/translation/session.rs`
- Modify: `src/providers/openai/translation/protocol.rs`
- Modify: `src/providers/openai/translation/commands.rs`
- Modify: `src/providers/openai/translation/events.rs`

- [ ] **Step 1: Mirror Task 5 for translation**

Same documentation pattern as realtime, emphasizing speech-to-speech / translation session purpose (used by `src/bin/transono.rs`).

- [ ] **Step 2: Phase 1 verification gate**

Run:

```bash
cargo doc --no-deps 2>&1 | tee /tmp/transono-doc.log
rg "missing documentation.*(core::|line::|providers::)" /tmp/transono-doc.log || true
```

Expected: **no** missing-docs lines for `core`, `line`, or `providers`. Warnings for `audio`, `runtime`, `ctl`, `console`, `testing` remain OK.

- [ ] **Step 3: Spot-check HTML**

Run: `cargo doc --no-deps --open` (or open `target/doc/transono/index.html`)

Check: crate root, `TranslationLine`, `Provider`, `OpenAITranslationProvider`.

---

## Phase 2 — Audio, runtime, ctl

### Task 7: `audio` module docs + traits / formats

**Files:**
- Modify: `src/audio/mod.rs`
- Modify: `src/audio/device.rs`
- Modify: `src/audio/input.rs`
- Modify: `src/audio/output.rs`
- Modify: `src/audio/audio.rs`
- Modify: `src/audio/encoded_audio.rs`
- Modify: `src/audio/pcm_audio.rs`
- Modify: `src/audio/audio_encoder.rs`
- Modify: `src/audio/encoders/mod.rs`
- Modify: `src/audio/encoders/pcm.rs`
- Modify: `src/audio/encoders/base64.rs`

- [ ] **Step 1: Module overview**

At top of `src/audio/mod.rs`:

```rust
//! Audio devices, buffers, encoding, and DSP pipelines.
//!
//! Hot paths should avoid allocation: prefer pooled frames/PCM and
//! slice-based processing. Device backends: [`cpal`], [`pipewire`].
```

Keep existing `pub mod` / `pub use` list unchanged.

- [ ] **Step 2: Document public traits and format types**

Document `AudioDevice`, `AudioDeviceFactory`, device config enums, `AudioInput`, `AudioOutput`, `Audio`, `AudioFormat`, `EncodedAudio*`, `PcmAudio`, encoders/decoders.

- [ ] **Step 3: Verify subset**

Run: `cargo doc --no-deps 2>&1 | rg "missing documentation.*audio::(device|input|output|audio|encoded)" || true`

---

### Task 8: Buffers, frames, pools, pipeline, processors

**Files:**
- Modify: `src/audio/frame.rs` — **translate Russian `//!`/`///` to English**
- Modify: `src/audio/frame_pool.rs` — translate
- Modify: `src/audio/audio_buffer.rs` — translate
- Modify: `src/audio/sample_buffer.rs`
- Modify: `src/audio/planar_sample_buffer.rs`
- Modify: `src/audio/pcm_pool.rs`
- Modify: `src/audio/pipeline.rs`
- Modify: `src/audio/processor.rs` — translate Russian trait docs
- Modify: `src/audio/processors/mod.rs` + all processors
- Modify: `src/audio/diagnost/**`

Russian → English examples (preserve meaning):

```rust
// frame_pool.rs was:
//! Предварительно выделенный пул аудиокадров.
// becomes:
//! Pre-allocated pool of audio frames.
//!
//! [`FramePool`] does not manage frame lifecycle; it only provides fast
//! access to memory. Ownership of [`FrameId`] is defined by the rtrb queues.
```

```rust
// processor.rs traits — English:
/// Stream-level audio processor (may work on [`Audio`] containers).
pub trait AudioProcessor: Send { ... }

/// DSP processor operating on the internal PCM representation.
pub trait DspProcessor: Send { ... }

/// Combined pipeline stage used by [`AudioPipeline`](crate::audio::AudioPipeline).
pub trait Pipeline: AudioProcessor + DspProcessor + Send { ... }
```

Add `# Examples` `no_run` or a small compiling example for constructing `AudioPipeline` / adding a `Processor` if feasible without hardware.

Document `LatencyStats`, `Pipelines`, public processor configs (`Normalizer`, `Compressor`, etc.).

- [ ] **Step 1: Apply docs + translations as above**
- [ ] **Step 2: Verify no Cyrillic left in audio rustdoc**

Run: `rg -n "///|//!" src/audio | rg "[А-Яа-яЁё]" || true`

Expected: empty.

---

### Task 9: CPAL + PipeWire backends

**Files:**
- Modify: `src/audio/cpal/**`
- Modify: `src/audio/pipewire/**`

- [ ] **Step 1: Module + type docs**

Document `AudioDevicesCpal`, `AudioInputCpal`, `AudioOutputCpal`, `PipeWireInput`/`Output`, `PipeWireWorker`, `WorkerConfig`, `PipeWireDeviceFactory`.

Note threading: PipeWire worker / callback constraints; no allocation in realtime callbacks where that is the design intent.

- [ ] **Step 2: Verify**

Run: `cargo doc --no-deps 2>&1 | rg "missing documentation.*audio::(cpal|pipewire)" || true`

Expected: empty.

---

### Task 10: `runtime` module (draft)

**Files:**
- Modify: `src/runtime/mod.rs`
- Modify: `src/runtime/input_port.rs` — replace Russian one-liner
- Modify: `src/runtime/output_port.rs` — replace Russian one-liner
- Modify: `src/runtime/mixer.rs`
- Modify: `src/runtime/link.rs`
- Modify: `src/runtime/splitter.rs` (document whatever is `pub`)

- [ ] **Step 1: Mark draft status in module docs**

```rust
//! Experimental audio-graph helpers (ports, mixer, links).
//!
//! **Status: draft** — APIs may change as the v0.6 runtime refactor lands.
//! Prefer [`crate::line::TranslationLine`] for production paths today.
```

Translate:

```rust
/// Adapter presenting this port as an [`AudioInput`](crate::audio::AudioInput) to external APIs.
/// Adapter presenting this port as an [`AudioOutput`](crate::audio::AudioOutput) for the audio API.
```

- [ ] **Step 2: Verify**

Run: `cargo doc --no-deps 2>&1 | rg "missing documentation.*runtime::" || true`

---

### Task 11: `ctl`, `console`, `testing`

**Files:**
- Modify: `src/ctl/mod.rs`, `backend.rs`, `pipewire.rs`, `windows.rs`, `state.rs`
- Modify: `src/ctl/commands/**`
- Modify: `src/console/mod.rs`
- Modify: `src/testing/mod.rs`, `src/testing/websocket_server.rs`

- [ ] **Step 1: Document ctl**

```rust
//! Virtual audio device control for meeting-app integration.
//!
//! Creates and inspects OS-level virtual mic/speaker pairs used to bridge
//! conference apps and the translator.
```

Document `Backend`, `DeviceSet`, `DeviceState`, `DeviceStatus`, `Doctor*`, `create_backend`, `PipewireBackend`, `WindowsBackend`, `VirtualAudioDevices`, and each `commands::*_::run`.

- [ ] **Step 2: Short docs for console + testing**

```rust
//! Terminal UI (ratatui) for interactive translation sessions.
//! In-process helpers for integration tests (e.g. mock WebSocket server).
```

Document `ConsoleApp`, `WebSocketTestServer`, and other `pub` items.

- [ ] **Step 3: Verify**

Run: `cargo doc --no-deps 2>&1 | rg "missing documentation.*(ctl::|console::|testing::)" || true`

---

### Task 12: Final Definition of Done gate

**Files:** none (verification only)

- [ ] **Step 1: Full missing-docs scan**

```bash
cargo doc --no-deps 2>&1 | tee /tmp/transono-doc-final.log
rg "missing documentation for" /tmp/transono-doc-final.log || true
rg -n "///|//!" src | rg "[А-Яа-яЁё]" || true
```

Expected:
- No `missing documentation for` on library `pub` items (binaries may still warn if built; prefer documenting only `lib` — `cargo doc -p transono --no-deps` documents the lib).
- No Cyrillic in `///` / `//!` under `src/`.

- [ ] **Step 2: Sanity build**

Run: `cargo check -p transono`

Expected: success; docs-only diffs.

- [ ] **Step 3: Update checklist in the design spec**

In `docs/superpowers/specs/2026-07-20-rustdoc-public-api-design.md`, mark Definition of Done items as done (documentation meta only).

- [ ] **Step 4: Ask user about committing**

Do not commit unless requested.

---

## Self-review (plan vs spec)

| Spec requirement | Task(s) |
|------------------|---------|
| English rustdoc on all `pub` library items | Tasks 1–11, gate 12 |
| Phased: core/line/providers then audio/runtime/ctl | Tasks 1–6 then 7–11 |
| `#![warn(missing_docs)]` | Task 1 |
| Depth B: Errors/Panics/Examples on entry points | Tasks 3–5 (`Provider`, `TranslationLine`, OpenAI) |
| Russian inline → English | Tasks 8, 10 (and any found in Phase 1) |
| No behavior changes | All tasks docs-only |
| No `deny(missing_docs)` | Task 1 uses `warn` only |
| Verify with `cargo doc` | Steps in each task + Task 12 |

**Placeholders:** none intentional. Example snippets may need constructor-name tweaks when implementing — Step 1 of Task 3 explicitly requires reading the real config API.

**Type consistency:** Uses existing names (`TranslationLine`, `Provider`, `Pipelines`, `CoreError`, OpenAI types as in tree).
