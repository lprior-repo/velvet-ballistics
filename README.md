# Velvet Ballastics

`velvet-ballastics` is a nightly-Rust, single-binary orchestration engine targeting raw in-memory workflow performance with Fjall-backed durability and no HTTP control plane in the hot path.

`/velvet-ballistics-MASTER.md` is the authoritative build plan, lifecycle, phase tracker, architecture contract, and implementation acceptance contract. Other docs provide goals and context only; they cannot override the master document.

## Canonical Naming

- Product, binary, and package: `velvet-ballastics`
- Crate and module: `velvet_ballastics`
- Bead rig: `velvet-ballastics`
- Bead database: `velvet_ballistics`
- Language version: `velvet-ballastics/v1`
- `velvet-ballistics` is invalid except in external migration artifacts.

## Architecture Spine

```text
YAML source
  -> strict parser and validator
  -> generated Rust maxperf mode
  -> compiled workflow IR
  -> numeric-slot RunFrame
  -> synchronous in-memory engine loop
  -> bounded memory/IPC ingress
  -> Fjall append-only journal
  -> cold observability projection
```

Runtime core excludes YAML, JSON, and HTTP. Every runtime transition must be bounded, numeric, and benchmarkable.

## Workspace Target

```text
crates/velvet_ballastics-core       hot in-memory engine and compiled IR
crates/velvet_ballastics-compiler   cold authoring/compiler boundary
crates/velvet_ballastics-ipc        bounded memory/IPC ingress primitives
crates/velvet_ballastics-storage    Fjall append-only journal boundary
crates/velvet-ballastics            binary entrypoint
benches/                            benchmark evidence for speed claims
```

## Workflow Commands

```bash
bd prime
bd ready
bd show <id>
bd update <id> --claim
bd close <id>
bd dolt push
moon ci
```

Use beads for all task tracking: create or claim beads before implementation, close or update them after completion, and use `bd remember` for persistent knowledge. Never use markdown TODOs.

## Moon And Beads

- `moon ci` is canonical. Prefer it over ad-hoc Cargo gates.
- Source lint is zero tolerance; tests compile and run without strict test clippy.
- Use Moon v2 configuration once scaffolded.
- Active beads Dolt remote: `https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics`, branch `main`.
- Do not commit `.beads/dolt`, `.beads/backup`, `.beads/embeddeddolt`, locks, or runtime database state.
- Embedded beads mode may require serial `bd` commands because only one writer can hold the lock.

## Engineering Rules

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg`.
- No unchecked indexing, slicing, casts, or arithmetic.
- Generated Rust mode is mandatory for maxperf execution.
- Every speed claim requires benchmark evidence.
