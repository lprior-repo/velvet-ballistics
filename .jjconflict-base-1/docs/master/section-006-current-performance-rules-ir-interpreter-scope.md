---
section: 6
title: "Current Performance Rules — IR Interpreter Scope"
parent: velvet-ballistics-MASTER.md
---

## 6. Current Performance Rules — IR Interpreter Scope


The current performance goal is a fast, bounded IR-interpreter backend. Rust workflow code generation, generated-vs-IR ratio targets, `maxperf` acceptance, PGO release workflows, and public maximum-throughput claims are removed from the current contract.

Current rules:

1. `CompiledWorkflow` IR is the required runtime execution artifact.
2. Runtime state is numeric and handle-based.
3. Hot loops must use checked table access, bounded stacks, bounded queues, and preallocated or reservation-checked frame state.
4. Deterministic steps run synchronously inside the shard loop until suspension.
5. No async task is spawned per step.
6. No text formatting, YAML parsing, JSON parsing, HTTP handling, or string reference resolution on hot execution paths.
7. Any optimization must include before/after benchmark output, benchmark metadata, and no correctness regression.
8. `target-cpu=native`, PGO, and generated workflow execution are not current semantic or release-engineering requirements.
9. Runtime architecture is shard-owned, single-server, synchronous deterministic execution until suspension.
10. Data layout is hot/cold split: hot state has numeric IDs and handles; cold side tables carry spans, names, YAML paths, messages, and diagnostics.
11. Queues and scheduling use bounded `ArrayQueue`/`rtrb`, explicit backpressure, and no task-per-step spawning.
12. Persistence uses Postcard binary records and Fjall keyspaces with bounded writer queues and explicit durability modes.
13. Compilation resolves strings, references, actions, accessors, constants, branches, and resource contracts before run admission.
14. Turbo-style admission admits a run only after required slots, step states, expression stacks, frame space, trace space, journal buffers, IPC buffers, and queue commands are preallocated or reserved; deterministic transitions must not allocate after acceptance unless a documented resource contract permits it.

---
