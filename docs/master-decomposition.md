# Master Document Decomposition

This document decomposes `velvet-ballistics-MASTER.md` into reviewable architecture decision families. The master document remains authoritative; this file is an index and review aid only.

## Current Milestone

The active milestone is Backend / IR Interpreter Complete.

Current-scope execution path:

```text
strict YAML source
-> cold parser and validator
-> expression compiler
-> bounded compiled numeric IR
-> accepted artifact with digests, budgets, contracts, and capabilities
-> runtime admission
-> shard-owned synchronous IR interpreter
-> Fjall journal, snapshots, replay, and recovery
-> direct Rust API, binary IPC, and CLI observability
-> evidence gates
```

Current-scope exclusions:

```text
runtime YAML interpretation
runtime JSON or HTTP in vb_core, vb_runtime, vb_storage, or vb_ipc
maxperf, PGO, or target-cpu native as current release gates
native Makepad UI or any UI implementation as a backend blocker
distributed consensus, replication, quorum, or leader election
```

## Decomposition Table

| Decision family | Master sections | ADRs | Review risk |
|-----------------|-----------------|------|-------------|
| Product identity and scope | 0, 22, 44, 68 | ADR-001, ADR-021 | Scope bleed into DAG runner, codegen, maxperf, or UI |
| Naming and repository shape | 1, 23, 34 | ADR-002 | Wrong crate names, root production files, migration spelling drift |
| Rust reliability governance | 2, 3, 4, 7, 52, 53 | ADR-003, ADR-024 | Panic surface, unchecked math, hidden allocation, unstable features |
| YAML and language validation | 8, 9, 10, 16, 25, 26 | ADR-004, ADR-005 | Runtime YAML, loose schema, vague diagnostics |
| Compiled artifact and IR | 14, 15, 51, 63 | ADR-006, ADR-011, ADR-016 | Unverified IR, digest ambiguity, raw submit bypass |
| Values, expressions, and taint | 11, 27, 46, 47, 48 | ADR-007, ADR-008, ADR-009 | F64 parity gaps, secret leaks, handle misuse |
| Bounded execution | 13, 20, 45, 56, 64 | ADR-010, ADR-011, ADR-024 | Per-primitive bounds without whole-run bounds |
| Runtime ownership | 20, 45, 53, 55, 62 | ADR-011, ADR-024 | Async core drift, task-per-step scheduling, shared mutable run maps |
| Actions and effects | 19, 47, 55, 65, 66 | ADR-012, ADR-016 | Idempotency overclaiming, capability bypass |
| Durability and recovery | 18, 49, 54, 61, 67, 68 | ADR-013, ADR-014, ADR-023 | Crash-safety claims without recovery evidence |
| IPC and operator interface | 21, 33, 50, 69, 75 | ADR-015, ADR-017, ADR-005 | JSON/HTTP ingress drift, weak CLI diagnostics |
| Assurance and evidence | 36, 37, 38, 39, 40, 43, 60, 77 | ADR-018, ADR-019, ADR-022 | Evidence laundering, placeholder benchmarks, toy proof models |
| Architectural drift | 67 | ADR-020 | Known gaps hidden by high-level docs |

## Existing Docs That Must Be Reconciled

These existing docs contain useful material but must remain subordinate to the master document and this ADR package:

| Existing doc | Problem to watch |
|--------------|------------------|
| `docs/runtime-architecture.md` | Uses stale product spelling and hyphenated crate names; must map to `vb_core`, `vb_compile`, `vb_ipc`, `vb_storage`, and `vb_runtime`. |
| `docs/language-spec.md` | Still says `Status: draft`, uses stale product/CLI/UI language, and contains UI north-star material outside current backend scope. |
| `docs/compiled-ir.md` | Describes expression bytecode and action IDs as future even though the master treats them as current mandatory coverage. |
| `docs/storage-journal.md` | Says recovery hydration is future while the master makes recovery/replay part of current DoD and drift closure. |
| `docs/rust-governance.md` | Still references PGO tasks; master section 41 removes PGO and maxperf from current release gates. |
| `docs/generated-workflows.md` | Correctly marks generated mode deferred, but any command examples remain future-only. |
| `docs/deferred-ui.md` | Correctly marks UI deferred; any UI claims remain non-blocking for Backend / IR Interpreter Complete. |

## Architecture Review Questions

Every future architecture or implementation bead should answer these before code changes:

1. Does the change preserve accepted artifacts as the production trust unit?
2. Does it keep YAML, JSON, HTTP, formatted text output, string lookup, and dynamic maps out of hot runtime core?
3. Does it introduce any unbounded queue, loop, retry, fanout, buffer, timer, page scan, persistence batch, or expression stack?
4. Does it weaken shard ownership, add async to core crates, or create task-per-step scheduling?
5. Does it bypass Fjall journal ordering, runtime admission, capability grants, or secret-presence checks?
6. Does it claim performance, crash safety, exact-once behavior, or proof closure without raw evidence?
7. Does it accidentally promote deferred codegen, maxperf, PGO, native CPU, or UI work into the current backend milestone?
