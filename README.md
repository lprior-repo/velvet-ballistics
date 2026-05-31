# velvet-ballistics

`velvet-ballistics` is a Rust-nightly, no-unsafe, no-panic, single-server durable execution engine for AI-authored workflows.

YAML is a cold authoring format only. Runtime execution uses accepted compiled IR, numeric state machines, bounded shard-owned resources, Fjall persistence, Postcard binary records, direct Rust API ingress, and binary IPC.

The current milestone is Backend / IR Interpreter Complete. Generated Rust execution, native UI, and maxperf release work are deferred history unless repo-root `velvet-ballistics-MASTER.md` explicitly reactivates them.

## Source Of Truth

- Repo-root `velvet-ballistics-MASTER.md`: authoritative architecture, lifecycle, phase tracker, naming, and acceptance contract.
- `AGENTS.md`: tiny auto-loaded agent harness.
- `docs/agent-harness-writeup.md`: rationale for the token-efficient agent setup.
- `docs/agent-skill-routing.md`: explicit skill triggers, non-triggers, and handoffs for this repo.
- `docs/agent-operating-guide.md`: on-demand agent workflow, beads, verification, shell safety, and closeout guide.
- `docs/`: task-specific architecture and operations references.

If docs conflict, repo-root `velvet-ballistics-MASTER.md` wins.

## Core Rules

- First-party Rust forbids `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic, ignored `Result`, and unbounded resource growth.
- Runtime core forbids YAML interpretation, JSON parsing, HTTP routing, dynamic string lookup, and task-per-step scheduling.
- Performance claims require real before/after benchmark evidence.
- Formal verification must bind to production behavior; toy Kani, Verus, or TLA+ proofs are not acceptance evidence.

## Workspace

```text
crates/                  production crates
crates/workspace_tests/  cross-crate integration tests and benchmarks
fuzz/                    fuzz targets
xtask/                   automation and tooling
docs/                    focused documentation
verification/            formal/refinement artifacts
```

This repository uses a pure virtual Cargo workspace. Do not add production code, tests, or benchmarks at the repository root.

## Common Commands

```bash
moon ci
bd ready
bd show <id>
bd update <id> --claim
bd close <id>
bd dolt push
```

`moon ci` is the canonical full quality gate. Use focused package checks while iterating, then report exact commands and outcomes.

## Current Runtime Shape

```text
YAML source
  -> strict cold parser and validator
  -> compiled numeric IR
  -> shard-owned synchronous interpreter until suspension
  -> bounded action, wait, retry, fanout, queue, and storage transitions
  -> Fjall journal/snapshots/indexes with Postcard records
  -> direct Rust API and bounded binary IPC ingress
```

## Task Tracking

This project uses `bd` beads backed by Dolt. Use beads for live work state; do not create markdown TODO lists. Active remote: `https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics`, branch `main`.
