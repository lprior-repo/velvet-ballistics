# Deferred Codegen / Maxperf Track

Rust workflow code generation, generated-vs-IR ratio targets, PGO release workflows, `target-cpu=native`, and public maxperf claims are outside the current Backend / IR Interpreter Complete milestone.

## Current Status

- Required runtime mode: compiled `CompiledWorkflow` IR interpreted by shard-owned synchronous runtime state.
- Current CLI compile target: IR only.
- Current performance evidence target: IR interpreter, storage, IPC, direct API, and scheduler.
- Current release blockers exclude generated Rust equivalence, generated compile-fail tests, generated-vs-IR benchmark ratios, PGO, and maxperf release gates.
- Any residual codegen or maxperf material is historical evidence or cleanup debt unless repo-root `velvet-ballistics-MASTER.md` explicitly reactivates it.

## Active Performance Rules

1. Optimize the IR interpreter, not generated execution.
2. Keep runtime state numeric and handle-based.
3. Keep queues, frames, expression stacks, IPC frames, trace rings, and persistence batches bounded.
4. Do not allocate in deterministic hot transitions after run admission; admission and explicitly resource-contracted paths must be bounded or reservation-checked.
5. Do not format text, parse YAML/JSON, route HTTP, or resolve string references on hot runtime paths.
6. Do not claim speed without before/after benchmark output, workload shape, host metadata, and regression threshold.

## Deferred Scope

The following are future tracks only:

1. `vb_codegen` as an active workspace crate.
2. `compile --emit rust` or equivalent generated Rust emission.
3. Generated Rust execution for accepted workflow artifacts.
4. Generated Rust semantic equivalence against IR execution.
5. Generated compile-fail fixtures forbidding unsafe, unwrap, expect, panic, unchecked operations, runtime YAML, JSON, HTTP, and runtime string lookup.
6. Maxperf profile acceptance gates.
7. PGO collection and optimized release workflows.
8. Public generated-mode speed claims.

## Reactivation Contract

Codegen or maxperf may return only through dedicated reactivation beads that update the master contract and prove:

- why IR interpreter performance is insufficient,
- which IR node families are accepted,
- how unsupported IR fails closed before emission,
- exact semantic parity with IR execution,
- identical journal, slot, taint, error, terminal result, and replay behavior,
- no first-party unsafe or panic paths,
- generated output passes formatting, linting, compile, behavior, and compile-fail gates,
- performance claims have real baseline/result evidence,
- rollback behavior if generated execution diverges,
- generated dependencies do not enter runtime core unless explicitly accepted by the master contract.

Until then, codegen and maxperf are not current release gates.
