---
section: 35
title: "Implementation Phases"
parent: velvet-ballistics-MASTER.md
---

## 35. Implementation Phases


Phase build order is mandatory. The old giant primitive phase is rejected; every primitive family has its own implementation, test, fuzz, and benchmark beads.

| Phase | Name | Required delivery |
|-------|------|-------------------|
| -1 | Name/repo rebaseline | Canonical spelling, folder/package/crate/bead rebaseline, migration notes. |
| 0 | Toolchain/lints/CI/Moon | Nightly pin, hard lints, Moon tasks, and optional advisory supply-chain reporting skeleton. |
| 1 | Core scalar types | IDs, `WorkflowId::as_u32`, `RunId::as_u64`, `FiniteF64`, errors, limits. |
| 2 | Runtime value arenas | `SlotValue` handles, symbol/list/object/blob arenas, taint arrays. |
| 3 | Strict YAML event parser | `saphyr-parser` wrapper, YAML profile rejection, source maps, fuzz. |
| 4 | AST | Typed workflow, trigger, step, primitive, expression, result AST. |
| 5 | Schema validator | Required/unknown fields, ID rules, primitive count, diagnostics. |
| 6 | Reference validator | Reference tables, future/direct runtime reference rejection. |
| 7 | Control-flow validator | CFG, forward `then`, reachability, cycle rejection. |
| 8 | Type/taint validator | Input/action/result types, secret taint, leak rejection. |
| 9 | Expression lexer/parser | Bounded expression grammar, operators, helpers, parse diagnostics. |
| 10 | Expression bytecode | `ExprProgram`, fixed stack, overflow/underflow tests. |
| 11 | Slot compiler | Slot layout, accessors, constants, symbol interning, digests. |
| 12 | Core IR | Final `CompiledNodeKind`, IR validator, resource contracts. |
| 13 | Minimal deterministic engine | `SetConst`, `Copy`, `Choose`, `ChooseSlot`, `Finish`, `StepBudget`, invariant tests. |
| 14 | Direct API | Submit, cancel, inspect, list events, answer ask, complete/fail action. |
| 15 | Fjall base storage | Keyspaces, keys, workflow source, compiled IR, run headers, blobs. |
| 16 | Binary journal | Postcard record envelope, event records, schema versions, writer queue. |
| 17 | Snapshots/recovery base | Snapshot format, snapshot-plus-tail recovery, corruption handling. |
| 18 | Action ABI | Compile-time `ActionId`, ticket/outcome model, static numeric dispatch. |
| 19 | `do` | Action suspension, completion/failure resume, journal integration. |
| 20 | `retry`/`try_again` | Bounded retry policies, delay state, exhaustion semantics. |
| 21 | `on_error`/`then` | Handler routing, typed error slots, forward transitions. |
| 22 | `for_each` | Bounded iteration, `at_once`, item slots, ordered output. |
| 23 | `together` | Bounded branches, branch state, joins, failure policy. |
| 24 | `reduce` | Accumulator slots, bounded iteration, deterministic reducers. |
| 25 | `repeat` | Attempts, checks, finish semantics, time/attempt bounds. |
| 26 | `collect` | Page/item/time limits, pagination state, finish materialization. |
| 27 | `wait`/`ask` | Timer wheel, ask tickets, answer validation, timeout recovery. |
| 28 | Shard scheduler | Run ownership, bounded queues, frame pools, cancellation, shutdown. |
| 29 | Binary trace/counters | Trace ring, counters, binary drain, overhead benchmarks. |
| 30 | Binary IPC | `mio` Unix socket loop, required commands, frame fuzzing. |
| 31 | CLI | Validate, verify, compile IR, explain, diff, simulate, run, run-compiled, submit, replay, inspect, events, incident, IPC serve, action/system/doctor/AI context, bench-run. |
| 32 | Full recovery/replay | Digest mismatch detection, full primitive replay, non-idempotent policy. |
| 33 | Full benchmark suite | Criterion/iai suites, metadata, IR interpreter latency/throughput, storage, IPC, direct API, scheduler. |
| 34 | Hardening | Full gates, sanitizer jobs, fuzz expansion, docs, bead evidence, Backend / IR Interpreter Complete readiness. |
| 37 | Whole-workflow boundedness | Static dataflow analyzer: compute `WholeWorkflowBudget` from IR, propagate bounds through nested loops/branches, reject if any budget exceeds policy. New `BoundednessPolicy` config. Tests: nested fanout, sequential sum, conditional max, unbounded rejection. Resolves DRIFT-3 (aggregate budget gap) with Phase 45. |
| 38 | Idempotency verification gate | `SideEffect` + `RetrySafety` classification per action. Verification gate rejects retry on side-effecting actions without idempotency key. Key ingredient validation (reject secrets, random, time in keys). New `IdempotencyViolation` error type. Tests: every side-effect class, key restriction, retry reachability. |
| 39 | Accepted artifacts + admission | `AcceptedArtifact` record with `VerificationProof`. `RunAdmission` flow: artifact digest, input validation, capability check, secret availability, `RunAccepted` event. Runs bind to artifact by digest, not loose YAML. CLI `--strict` mode for AI-authored workflows. Tests: admission rejection paths, artifact binding, strict-mode warnings. |
| 40 | Evidence chain completion | Slot value/taint snapshots in journal. Action input/output payload persistence for completed actions. Durability proof per primitive (each primitive must document what journal events constitute proof of completion). `VerificationProof.durable` field gates acceptance. Tests: crash recovery with evidence chain, payload reconstruction. |
| 41 | Capability model | `Capability` struct. Actions declare required capabilities. Admission checks granted capabilities. `CapabilityDenied` rejection. Operator grants capabilities at run submission. Tests: missing capability rejection, granted capability acceptance. |
| 42 | Validation deduplication | Eliminate duplicate validation between `vb_validate` and `vb_compile`. Single validation pipeline operating on a shared intermediate representation. Both crate APIs preserved for backward compatibility but backed by one implementation. Resolves DRIFT-5. |
| 43 | Taint propagation fix | Fix runtime taint tracking: `EvalExpr` joins taint from loaded slots, `BuildObject`/`BuildList` join taint from field/item slots, `Finish` carries taint in signal. Expression evaluator returns `(SlotValue, Taint)` pairs. Compile-time checks remain as defense-in-depth. Resolves DRIFT-1. |
| 44 | Recovery evidence chain | Emit `SlotWritten` + `StepStarted`/`StepSucceeded` for every deterministic step. Gate hydration on `UnsupportedRecoveryState` — fail with typed error if slots/taint cannot be reconstructed. Replace `Ok(()) \| Err(_) => {}` pattern in shard with propagated errors. Resolves DRIFT-2. |
| 45 | Resource budget enforcement | Per-run `ValueStore` arena cap. Tightened `ResourceContract` defaults (no `u16::MAX`). Hard ceiling on `StepBudget` per tick. Replace Collect global Mutex with per-run state. Resolves DRIFT-3. |
| 46 | IR structural validation | `try_from_parts` validates reachable nodes, forward-only edges, loop pairing, SymbolId ranges, accessor path segments. Artifact loading treats input as untrusted. Resolves DRIFT-4. |

Round 2 current implementation state, observed in this tree and not a final release claim:

| Area | Round 2 state | Remaining gap before backend DoD |
|------|---------------|--------------------------------|
| Naming/workspace | Canonical crate layout and package spelling are represented in the workspace. | Mechanical spelling gates and bead evidence still decide acceptance for future changes. |
| Core/value/IR | `vb_core` exposes numeric IDs, handle-based `SlotValue`, `ValueStore`, taint/state APIs, bounded expression/accessor evaluation, resource contracts, and deterministic transition surfaces. | Full final primitive semantics still require end-to-end compiler/runtime/replay evidence. |
| YAML/validation/compile | Strict YAML parsing, AST validation, reference/control/type-taint checks, slot/accessor/constant APIs, digesting, artifact emission, and mandatory lowering function surfaces exist. | Source-to-IR lowering must be proven for the full v1 primitive set, not only constructor/API coverage. |
| Expression engine | Lexer/parser/typecheck/bytecode surfaces exist with bounded execution contracts. Store-aware helper implementations exist for the current interpreter surfaces. | Helper type/evaluator parity, F64 mixed/coercion behavior, and mutation resistance still require gate evidence. |
| Storage/recovery | `vb_storage` exposes required keyspace names, key encoders, record envelope encode/decode, journal writer queue, snapshots, replay helpers, recovery summaries, and frame-seed hydration for slot values/taint/step states. | Pending-action hydration, strict persistence-before-ack behavior, digest mismatch coverage, and end-to-end crash recovery evidence remain release gates. |
| Runtime/direct API | `vb_runtime` exposes direct API, shard/frame-pool/action/wait/ask/trace/counter surfaces, admission/capability surfaces, and typed runtime errors. | Strict persistence-before-ack behavior, shutdown/cancellation edge cases, pending-action recovery, and full lifecycle evidence remain gates. |
| IPC | `vb_ipc` exposes bounded frame/header/payload validation, typed payloads, memory ingress, client/server surfaces, and required command handlers. | Socket-loop fuzz/backpressure evidence and runtime integration gates remain required. |
| Removed codegen/UI | `vb_codegen`, `vb_ui_model`, and `vb_ui_makepad` are removed from active workspace scope. | They are not current acceptance gates and must not block Backend / IR Interpreter Complete. |
| Tests/audits | Error-variant completeness and diagnostic-code range tests exist; companion docs record benchmark and dependency policy constraints. | Full matrix gates, fuzz, Miri, coverage, mutants, sanitizer, benchmark metadata, and bead closure evidence are still required. Supply-chain/dependency reports are advisory under the owner waiver. |

Round 2 status rule: a public function existing in a crate is only API surface evidence. It is not proof that the phase is complete unless the required tests, fuzz/property coverage, benchmark evidence where applicable, and bead closure evidence have actually passed.

---
