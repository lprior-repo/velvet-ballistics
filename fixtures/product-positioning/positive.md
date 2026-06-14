# Velvet Ballistics Overview

velvet-ballistics is a single-server durable execution engine that
verifies AI-authored workflows before admission, persists an inspectable
journal, protects side effects with idempotency evidence, and enforces
resource and taint bounds.

## What it is

- AI-safe
- Local-first
- Numeric state machines
- Numeric slots
- Numeric actions
- Shard-owned state
- Deterministic synchronous execution until suspension

## What it is not

It is a verification-first runtime, not a graph editor. The unit of trust
is the accepted artifact, not the source.

## Workspace

- `crates/vb_core` Compiled IR, engine, frame, value store, diagnostics
- `crates/vb_yaml` YAML parser, AST, source maps
- `crates/vb_validate` Control-flow, reference, schema, taint validation
- `crates/vb_compile` Full compilation pipeline (YAML to validated IR)
- `crates/vb_storage` Fjall journal, envelope, recovery, snapshots
- `crates/vb_runtime` Shard engine, action dispatch, primitives, frame pool
- `crates/vb_ipc` Unix domain socket server/client, binary protocol
