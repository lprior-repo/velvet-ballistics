---
section: 0
title: "Prime Directive"
parent: velvet-ballistics-MASTER.md
---

## 0. Prime Directive


`velvet-ballistics` is a Rust-nightly, no-unsafe, no-panic, single-server, ultra-low-latency durable execution engine for workflow orchestration. YAML is an authoring format only. The runtime never interprets YAML, parses JSON, serves HTTP, or routes text commands. Workflows compile into numeric state machines over numeric slots, numeric actions, numeric steps, and bounded resource contracts.

The current implementation goal is **Backend / IR Interpreter Complete**: strict YAML authoring, validation, verification, compiled numeric IR, IR-interpreter execution, Fjall durability, direct Rust API, binary IPC, CLI observability, replay/recovery, and evidence gates. Rust workflow code generation, `maxperf` acceptance, and all native UI/Makepad work are removed from the current core feature set. Residual codegen/UI/maxperf material is cleanup debt unless it is explicitly quarantined as historical evidence.

The runtime uses numeric state machines, numeric slots, numeric actions, shard-owned state, and deterministic synchronous execution until suspension. Fjall is required for persistence. Postcard is required for compact binary records. Ingress is direct Rust API plus binary IPC. `CompiledWorkflow` IR is the only active execution artifact for the current milestone. Any section explicitly marked removed, historical, or quarantined is non-normative for the current milestone and cannot block Backend / IR Interpreter Complete acceptance.

### Product Positioning Contract

Publicly, `velvet-ballistics` must not be described as a generic DAG runner, low-code graph editor, YAML-as-programming framework, Airflow replacement, or Temporal clone. Those frames hide the actual wedge and invite false comparisons.

The product identity is: an AI-safe, local-first, single-server durable execution engine that verifies AI-authored workflows before admission, persists an inspectable journal, protects side effects with idempotency evidence, and enforces resource and taint bounds. Generated Rust execution is not a current product path.

The unit of trust is the accepted artifact, not the YAML source. YAML is a cold authoring surface. Verification certificates, compiled IR digests, resource budgets, action contracts, capability grants, journals, snapshots, and replay reports are the operational truth.

Competitive comparison is allowed only with scope discipline:

1. Compare durability and replay semantics to Temporal, DBOS, and AWS Step Functions.
2. State the v1 single-server boundary plainly: no replication, no quorum, no leader election, no distributed control plane.
3. Compare data orchestration ergonomics to Airflow and Dagster only when explaining non-goals.
4. Never claim production readiness, performance superiority, or crash safety without executable evidence and benchmark/recovery artifacts attached to the bead or release.
5. The public demo path is `verify -> simulate -> submit -> incident/replay`, not drawing a DAG on a canvas.

The final product must provide all of the following. None are optional:

1. Rust nightly toolchain with mechanical lint gates.
2. First-party code forbids `unsafe`, `unwrap`, `expect`, `panic`, unchecked indexing, unchecked slicing, unchecked casts, unchecked arithmetic, ignored `Result`, and unbounded resources.
3. YAML authoring only through a strict parser and validator.
4. No runtime YAML, JSON, or HTTP in `vb_core`, `vb_runtime`, `vb_storage`, or `vb_ipc`.
5. Compiled numeric workflow IR with `WorkflowId`, `StepIdx`, `SlotIdx`, `ExprIdx`, `ActionId`, `AccessorIdx`, `ConstIdx`, and bounded tables.
6. Handle-based runtime values using interned symbol/list/object/blob handles and finite numeric values.
7. Deterministic state-machine execution until suspension on action, wait, ask, retry, fanout join, queue admission, or storage policy boundary.
8. Shard-owned run state with bounded queues, bounded frame pools, bounded trace rings, bounded retries, bounded fanout, bounded expression stacks, bounded IPC frames, and bounded persistence batches.
9. Fjall persistence for workflow source, compiled IR, run headers, journal events, snapshots, blobs, and indexes.
10. Postcard encoding for internal journal, snapshot, IPC payload, and compiled artifact records.
11. Direct Rust API ingress for fastest local embedding.
12. Binary IPC ingress for external local processes.
13. IR-interpreter execution is the required runtime mode for the current milestone.
14. Typed validation, compile, runtime, IPC, and storage failures.
15. Benchmarked optimizations only; no speed claim without measured before/after data.
16. AI changes are accepted only with actual evidence that the relevant formatting, linting, tests, fuzzing, recovery, benchmark, and CI reproducibility gates ran and passed; merely adding or naming a task is not acceptance evidence. Dependency/supply-chain/API reports are advisory under the 2026-05-23 owner waiver unless a separate bead explicitly makes a specific report blocking.

HTTP/JSON exclusion rule: HTTP and JSON are excluded from the v1 runtime core. Any future adapter must be a separate cold-path adapter crate and must not enter `vb_core`, `vb_runtime`, `vb_storage`, or `vb_ipc`.

---
