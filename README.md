# velvet-ballistics

A formally bounded workflow runtime for AI agent orchestration.

**TigerBeetle's engineering discipline applied to LangGraph's market.**

velvet-ballistics is a nightly-Rust, single-binary orchestration engine that verifies YAML-authored workflows into compact numeric IR, executes accepted IR through a bounded interpreter, dispatches native actions, and persists events through Fjall-backed append-only storage. No HTTP, no JSON, no async runtime in the hot path. Every transition is bounded, numeric, and benchmarkable.

## Why This Exists

Existing workflow engines (Temporal, Inngest, Prefect, BullMQ) share a common trait: they trust the runtime to behave. They don't bound memory, don't track information flow, and often keep source-like workflow structure in the runtime. This works for traditional workflows but fails for AI agent orchestration where:

- **Agent loops can explode** — unbounded retry, fan-out, and list processing need hard resource limits, not soft timeouts
- **Secrets leak through control flow** — a secret-tainted value choosing which public branch runs is an information channel
- **Runtime ambiguity compounds** — agent workflows need accepted artifacts, replayable evidence, and explicit resource budgets before execution

velvet-ballistics addresses all four dimensions:

| | velvet-ballistics | Temporal | LangGraph |
|---|---|---|---|
| Formal resource bounds | Checked arithmetic, bounded frames, slot budgets | Timeouts only | No |
| Taint tracking | Clean/DerivedFromSecret/Secret lattice | No | No |
| IR compilation | `CompiledWorkflow` numeric artifact with exact semantics | Interpreted history/worker model | Interpreted graph model |
| Runtime boundary | IR interpreter only; no runtime YAML/JSON/HTTP | Service/runtime boundary | Python object graph/runtime |

## Architecture

```text
YAML source
  -> strict compile-time parser
  -> validated AST
  -> typed expression bytecode (Pratt parser, 64-entry fixed stack)
  -> numeric slot compiler
  -> compact IR (34 node kinds, u16 step indices, u16 slot indices)
  -> bounded IR interpreter
  -> shard-owned in-memory runtime (no async, no allocation in hot path)
  -> native ActionId dispatch with taint enforcement
  -> Fjall binary persistence (9 keyspaces, blake3+crc32c envelopes)
  -> Unix domain socket IPC (bounded queue, 256 concurrent clients)
  -> SPSC trace ring (rtrb, 4096 events)
```

## Workspace

```text
crates/vb_core         Compiled IR, engine, frame, value store, diagnostics
crates/vb_yaml         YAML parser, AST, source maps
crates/vb_validate     Control-flow, reference, schema, taint validation
crates/vb_expr         Expression lexer, parser, bytecode, typecheck
crates/vb_compile      Full compilation pipeline (YAML -> validated IR)
crates/vb_storage      Fjall journal, envelope, recovery, snapshots
crates/vb_runtime      Shard engine, action dispatch, primitives, frame pool
crates/vb_ipc          Unix domain socket server/client, binary protocol
crates/workspace_tests Cross-crate integration tests and benchmark harnesses
fuzz/                  Parser, decoder, and IR fuzz targets
```

## Safety Guarantees

- **`#![forbid(unsafe_code)]`** across all crates
- Checked arithmetic everywhere (`deny(arithmetic_side_effects)`)
- No `unwrap`, `expect`, `panic`, `todo`, or `unimplemented`
- No unchecked indexing, slicing, or `as` conversions
- Bounded expression stacks (64 entries), bounded IPC frames, bounded trace rings
- Every runtime transition has a defined error, a defined step state, and a defined journal event

## Getting Started

```bash
# Build/type-check
moon run :check

# Test
moon run :test

# Lint (zero tolerance)
moon run :lint-src

# Canonical gate
moon ci

# Benchmark harness build only; performance claims require recorded evidence
moon run :bench-build
```

## Documentation

- `/velvet-ballistics-MASTER.md` — authoritative architecture contract, phase tracker, and implementation acceptance criteria
- `CHANGELOG.md` — release history and notable changes
- `RELEASE_CHECKLIST.md` — steps for future releases
- 62 normative sections covering runtime semantics, expression grammar, taint lattice, journal schemas, IPC transport, security threat model, and more

## Task Tracking

This project uses [beads](https://github.com/nicholasgasior/beads) for task tracking.

```bash
bd prime          # Load workflow context
bd ready          # Find available work
bd show <id>      # Review issue
bd update <id> --claim  # Claim it
bd close <id>     # Mark done
```

Active beads Dolt remote: `https://doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics`, branch `main`.

## License

MIT OR Apache-2.0
