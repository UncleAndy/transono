# Role & Communication Profile

- **Language**: You must communicate with the user strictly in Russian.
- **Tone**: Professional, technical, concise.

# Project Overview

This repository contains a dual-purpose Rust project:
1. A synchronous voice/text translator (real-time processing).
2. A high-performance audio processing library.

# Core Engineering Principles

You must strictly adhere to the following principles when writing, refactoring, or reviewing code:

## 1. Clean Architecture & SOLID
- **Single Responsibility**: Separate audio I/O, translation logic, and data transformation.
- **Open/Closed**: Design traits for extensibility (e.g., generic audio sources or translation backends).
- **Liskov Substitution / Interface Segregation**: Keep traits small and focused. Do not force implementations of unused methods.
- **Dependency Inversion**: High-level translation orchestration must depend on abstractions, not concrete audio devices.

## 2. Performance & Memory Management (Critical Sections)
For audio processing, buffer management, and hot paths, optimize for low latency:
- **Zero-Copy**: Pass data via slices `&[u32]`, `&[f32]`, or `&[u8]` and reference-counting (`Arc`) where applicable. Avoid copying buffers.
- **No-Allocation**: Do not allocate on the audio thread. Avoid `Vec::new()`, `String`, or `Box` inside tight loops. Use pre-allocated rings or fixed-size buffers.
- **Lifetimes**: Utilize explicit Rust lifetimes to safely pass references without cloning.

# Tech Stack & Guidelines

- **Language**: Rust (Latest Stable).
- **Concurrency**: Thread-safe audio handling (`Send + Sync`), atomic operations for state synchronization.
- **Error Handling**: Use the project's internal custom error types and the internal `Result` type alias (e.g., `crate::error::Result`). Never use standard standard `unwrap()` or `panic!` in production code. Prefer leveraging the project's specific error variants for robust error propagation.

# Expected Output Format

When generating Rust code, ensure:
1. Idiomatic Rust code layout.
2. Standard `clippy` lints are satisfied.
3. Doc-comments on public interfaces explaining lifetime bounds or unsafety (if any).
