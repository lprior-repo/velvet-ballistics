---
section: 44
title: "Backend / IR Interpreter Definition of Done"
parent: velvet-ballistics-MASTER.md
---

## 44. Backend / IR Interpreter Definition of Done


The current `velvet-ballistics` backend milestone is done when all 24 points are satisfied:

1. Canonical spelling is enforced for product, binary, package, crate/module, bead rig, bead database, and language version.
2. Any `velvet-ballistics` spelling outside the exact allowlist for `/home/lewis/src/Velvet-ballistics`, `/velvet-ballistics-MASTER.md`, or explicitly labeled pre-existing external migration artifacts is rejected.
3. Every primitive validates, compiles, runs, persists, recovers, and replays.
4. v1 supports both `manual` direct API submission and `ipc` binary IPC submission.
5. Runtime never interprets YAML and recovery never reparses YAML for existing runs.
6. JSON and HTTP are absent from `vb_core`, `vb_runtime`, `vb_storage`, and `vb_ipc`.
7. Runtime state uses numeric workflow, run, action, step, slot, expression, accessor, constant, and sequence IDs.
8. Action dispatch uses numeric `ActionId`; no runtime string action lookup exists.
9. Hot values use handle-based `SlotValue` with `SymbolId`, `ListId`, `ObjectId`, `BlobId`, and finite numbers.
10. Each run is owned by exactly one shard; no global mutable run map exists.
11. Queues, stacks, buffers, retries, fanout, timers, traces, batches, IPC frames, and resource contracts are bounded.
12. Turbo-style admission preallocates or reserves hot resources; deterministic transitions allocate nothing after acceptance unless a documented resource contract permits it.
13. Fjall stores workflow source, compiled IR, run headers, journals, snapshots, blobs, and indexes with magic/schema/version/kind/length envelopes.
14. Recovery and replay detect workflow, action, and policy digest mismatch and fail typed without default substitution.
15. Direct API implements submit, inspect, cancel, list events, answer ask, complete action, fail action, drain trace, health, and shutdown equivalents.
16. Binary IPC implements `SubmitRun`, `SubmitRunInline`, `CancelRun`, `InspectRun`, `ListEvents`, `AnswerAsk`, `CompleteAction`, `FailAction`, `DrainTrace`, `Health`, and `Shutdown`.
17. IR-interpreter execution covers every active final IR node and is the accepted execution mode.
18. Diagnostics include stable code, path, source span, message, and cold side-table context.
19. Validation, compile, runtime, storage, IPC, action, and replay failures are typed and graceful.
20. Forbidden constructs are absent: `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, ignored `Result`, runtime maps, hot formatting, runtime YAML/JSON/HTTP, and string reference/action lookup.
21. Unchecked indexing, slicing, casts, and arithmetic are absent from first-party code.
22. Every speed claim is backed by real benchmark evidence with p50/p95/p99, instruction counts, allocation counts, bytes allocated, latency, durability mode, and fixture metadata; compileable scaffold placeholders do not count.
23. Full current-scope gates pass: fmt, clippy hard denies, tests, nextest, Miri, coverage, fuzz smoke, mutants smoke, feature powerset, docs, benchmark build, storage/recovery evidence, IPC evidence, and direct API evidence. Supply-chain/dependency unsafe reports are advisory under the owner waiver unless a bead opts in.
24. Every phase parent bead, function-cluster child bead, fuzz target bead, benchmark bead, and P0 blocker bead in the current backend scope is closed with evidence, and mechanical gates can accept AI changes without human guesswork only when the relevant executable checks, tests, benchmarks, and bead evidence have actually run and passed.

---
