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


## Required activation and state boundaries

Rearm occurs only after the unique enabled owning latch successfully takes its validated `control -> body` backedge. Exhaustion, done, error and suspension paths never rearm. Only completed step states in inclusive `[body, control]` may change; owner and done are excluded. Slots, taints, counters, iterators, pending actions and wait resources are never cleared or rewritten by this helper.

Existing primitive body/latch resets must be routed through this helper so no candidate body/latch state mutation occurs before both-pass preflight. Primitive slot and PC writes are separate preceding effects, with no rollback claim. Cold permitted exits are exactly the owning declared done or an actual terminal `Finish` target; no open-ended terminal category. Nested regions may structurally validate but remain unsupported for activation until cleanup/state semantics are established. Exactly one enabled owning latch is required; multiple-latch semantics need a separate contract.


## Cold candidate integration, V96

The cold ownership/cache implementation is now integrated as a candidate at the exact source pin in `native-cold-v98/README.md`. The hot helper remains unimplemented at this receipt. The original PROPOSED header describes unaccepted overall delivery, not absence of this cold candidate. Constructor/cache tests31 and source-length pass; existing compatibility fixture failures remain open and preserved. Region errors run after numeric/reference/reachability/forward validation and before budget/input-slot validation; no universal multiply-invalid error precedence claim is made.

Only the owning Start kind body edge permits outside ingress; outer next/on_error to the same endpoints do not qualify. Edge provenance is retained. Internal nonterminal on_error recovery or ErrorHandler within an enabled region returns LoopRegionUnsupported; direct error exits to the exact owning done or actual Finish are permitted. These restrictions do not waive eventual full F1 recovery support. Nested topology validation does not authorize nested hot activation.


## Public primitive compatibility boundary (approved design, pending implementation)

Low-level public Next/Check functions retain their legacy entry-only reset behavior. The validated driver must explicitly call private deferred counterparts, sharing each decode/write/exhaustion implementation exactly once via statically selected LegacyEntry/DeferredOwned marker policies. Only the productive branch invokes that policy; exhaustion never resets even if raw body/done targets alias. No PC-equality inference, duplicate state machine, or legacy fallback after missing/mismatched cache is permitted. Legacy APIs do not promise whole-region preflight. Driver rearm retains the full-region contract. Existing nine legacy reentry tests remain unchanged.

Actual nested activation must be rejected by cold-derived indexed metadata at begin_drive_step after budget/PC lookup and before StepStarted/Running or primitive effects. Skipped Pending nested entry nodes do not by themselves prohibit an outer flat activation. Executed nested entry nodes remain unsupported until cleanup/state semantics are proven. Cold nested topology can remain valid without authorizing execution.
