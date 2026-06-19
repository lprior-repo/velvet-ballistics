# Loom Unsafe Waiver

## Status
APPROVED per Holzman `nightly_max_performance_waiver` template.

## Rule being waived
Holzman Rust §2: "No `unsafe` in production code by default."

## Justification
[Loom](https://github.com/tokio-rs/loom) provides deterministic permutation
testing for concurrent code. Loom's internals use `unsafe` for atomic
operations and thread/future spawning. This is upstream-maintained code, not
first-party production code.

## Scope
- Loom is an optional dev-dependency only (`loom = { version = "0.7",
  optional = true }`).
- Every Loom harness module is gated behind `#[cfg(loom)]` so the default
  build (and the production build) does not link Loom and does not execute
  any Loom `unsafe`.
- Production builds remain `#![forbid(unsafe_code)]` per
  `crates/vb_runtime/src/lib.rs` and `crates/vb_storage/src/lib.rs`.
- `cargo geiger` reports 0 first-party `unsafe` blocks.

## Evidence

### Loom Version
- Loom 0.7.2 (per `Cargo.lock`).
- Loom source: `registry+https://github.com/rust-lang/crates.io-index`.

### cfg(loom) Gating Inventory
Every Loom model module is gated behind `#[cfg(loom)]`:

- `crates/vb_runtime/src/models/loom/mod.rs:9` — `pub mod bounded_queue;`
- `crates/vb_runtime/src/models/loom/mod.rs:12` — `pub mod timer_fired_cancel;`
- `crates/vb_runtime/src/models/loom/mod.rs:15` — `pub mod shutdown_drain;`
- `crates/vb_runtime/src/models/loom/mod.rs:18` — `pub mod action_completion_cancel;`
- `crates/vb_runtime/src/models/loom/mod.rs:21` — `pub mod journal_writer_queue;`
- `crates/vb_runtime/src/models/loom/mod.rs:24` — `pub mod idempotency_retry_eviction;`

### Run Commands
Default build (no Loom): `cargo build -p vb_runtime --release`.

Loom-enabled build (dev only):
`RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --models`.

### Workspace Lint Policy
`Cargo.toml:70` lints the following `cfg` flags at deny level:
`'cfg(loom)', 'cfg(verus)', 'cfg(kani)'`. The Loom-related features are
declared in:
- `crates/vb_runtime/Cargo.toml:20` — `loom = { version = "0.7", optional = true }`
- `crates/vb_runtime/Cargo.toml:38` — `loom = ["dep:loom"]`
- `crates/vb_storage/Cargo.toml:28` — `loom = { version = "0.7", optional = true }`
- `crates/vb_storage/Cargo.toml:49` — `loom-vb-mrwe-7 = ["dep:loom"]`

## Review
Re-evaluate when:
- Loom is upgraded (currently pinned to 0.7).
- A first-party `unsafe` block is added to a production source file.
- A new Loom model is added without `#[cfg(loom)]` gating.
