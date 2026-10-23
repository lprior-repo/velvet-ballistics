# Rust Contract: Loop Body Rearm (Proposed, Not Implemented)

**Version:** 3.7  
**Status:** PROPOSED — not implemented, not release acceptance  
**Module:** `vb_runtime::engine::drive::loop_rearm` (proposed)

## API

```rust
pub(super) fn loop_body_rearm(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    control: StepIdx,
    body: StepIdx,
) -> RuntimeEngineResult<()>;
```

Called after primitive writes, evidence recording, and control step reaching `Succeeded`.

## Cold Constructor Proofs

Before any hot-path invocation, the constructor must prove:

- Unique owner, latch family, body, done, and state-slot matches
- STRICT index ordering: `start < body < control < done`
- `[body, control]` has single entry
- Workflow entry never inside loop region
- Incoming edges: internal or own `Start -> body`
- Outgoing edges: internal or declared owned `done`/terminal exits
- Complete successors for `next`, `on_error`, `Jump`, `Choose` (both forms/otherwise)
- All loop body/done/exhausted paths accounted for
- `Together` branches/entry/join validated
- `ErrorHandler` body/handler validated
- Async continuations validated

## Region Invariants

- Regions wholly nested or disjoint; no overlap or shared ownership
- No backedges except own latch and independently validated nested ones
- `WorkflowError::LoopRegionCrossing` validated before runtime
- Hot pair checks cannot replace cold closure proofs

## Hot-Path Semantics (No-Alloc, Two Passes)

1. **Preflight:** Validate all states BEFORE helper mutates
2. **Transition:** Only `Succeeded -> Pending`; `Pending` preserved
3. **Reject:** `Running`, `Waiting`, `Asking`, `Failed`, `Cancelled`, `Skipped` MUST reject unless separately proven recovery

No rollback claim for prior primitive writes or evidence. Nearest `Start` alone insufficient.

## Unsupported Until Cleanup/State Proof

- Nested body activation
- Recovered-error body activation

Full F1 must eventually support these; this contract is NOT release acceptance.

## Not Blanket Rejected

Normative suspend/error workflows outside enabled repeated scope.

## Test Requirements

- Outside entry body/check
- Crossing `Choose`/`on_error`
- Shared regions
- Internal `Jump` bypass
- Exact 2 `Set` max 1/max 3 trace + slot/taint sentinel/unchanged IR
- Inflight failure atomic helper

Existing two `Source` cases RED exactly one `Set` restriction.

## Related

- Historical V65 escape/reentry runtime RED; separate issue
- 3.8 Jump direction guard closes one escape ONLY, not region proof

## Non-Claims

- No performance claim
- No public `WorkflowParts` schema change

This revision supersedes earlier nearest-owner-only and preserve-failed/skipped language. It records independent design findings before any rearm implementation. The proposed error/API names are not implemented declarations.
