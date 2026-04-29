# Velvet Ballistics

Velvet Ballistics is a nightly-Rust, single-binary orchestration engine scaffold optimized for low-latency in-memory execution with Fjall as the append-only durability log.

The product constraint is explicit: no HTTP control plane in the hot path. YAML is an authoring format only. Runtime execution uses compiled numeric workflow state, bounded queues, preallocated run frames, and binary journal records.

## Architecture Spine

```text
YAML source
  -> strict parser and validator
  -> compiled workflow IR
  -> numeric-slot RunFrame
  -> synchronous in-memory engine loop
  -> bounded memory/IPC ingress
  -> Fjall append-only journal
  -> cold observability projection
```

## Workspace

```text
crates/vb-core              hot in-memory engine, compiled IR, slot values
crates/vb-compiler          native Rust YAML cold compiler boundary
crates/vb-ipc               bounded memory ingress primitives, no HTTP
crates/vb-storage           Fjall append-only journal boundary
crates/velvet-ballistics    binary entrypoint
docs/                       runtime and performance contracts
benches/                    benchmark placeholders
```

## Discipline

First-party source code is governed by JPL-Rust discipline:

- No `unsafe`.
- No `unwrap`, `expect`, panic paths, `todo`, or unchecked indexing.
- No unbounded queues, retries, fanout, or task spawning.
- Every runtime transition is numeric, bounded, and benchmarkable.
- Third-party `unsafe` is allowed only through audited dependency policy.

## Commands

```bash
cargo +nightly fmt --check
cargo +nightly check --workspace --all-targets --all-features
cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings
cargo +nightly test --workspace --all-features
cargo +nightly build --profile maxperf
```
