# ADR 011 (v1): Runtime and Node Semantics

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

The IR interpreter is the normative execution mode. Every `CompiledNodeKind` variant has defined behavior for inputs, slots, taint, step state, journal events, suspension, next program counter, resource checks, errors, action tickets, and replay behavior.

## Invariants

- Step states transition only through master-approved transitions.
- Invalid transitions return `InternalInvariantViolation`.
- Deterministic execution runs synchronously until suspension or terminal signal.
- No task-per-step scheduler exists in core runtime.
- No async function appears in `vb_core`, `vb_runtime`, `vb_storage`, or `vb_ipc`.

## Consequences

- Each node kind needs behavior tests and recovery/journal evidence where applicable.
- Adding a node kind is a language, IR, runtime, journal, diagnostics, and evidence change.

## Master Anchors

- Section 20: Runtime and Shard Design
- Section 45: Normative Runtime Semantics
- Section 55: Action Worker Model and Shard Non-Blocking Contract
- Section 62: No-Async Rule
