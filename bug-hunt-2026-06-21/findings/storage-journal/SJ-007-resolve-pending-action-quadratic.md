# SJ-007: `resolve_pending_action` performs O(N) removal → O(N²) incident scan

- **Severity**: Low
- **Category**: perf
- **Location**: `crates/vb_storage/src/journal/incident/model/analysis.rs:240`
- **Confidence**: confirmed

## Description

`resolve_pending_action` drains the `pending_scheduled_actions` vector using
`Vec::retain`, which is O(N) per call. `record_completed_action` and
`record_failed_action` both invoke it for every resolution event. Over a run
with S scheduled actions and R resolutions, the total cost is O(S·R), which
is O(N²) when most scheduled actions eventually resolve.

## Evidence

```rust
fn resolve_pending_action(pending: &mut Vec<SideEffectEvidence>, resolved: SideEffectEvidence) {
    let key = ActionEvidenceKey::from_evidence(resolved);
    pending.retain(|candidate| !key.matches(*candidate));
}
```

Callers (`record_completed_action`, `record_failed_action`,
`record_action_completion_envelope`) push to `pending_scheduled_actions` on
schedule and call `resolve_pending_action` on completion/failure. The
`IncidentAnalysis.pending_scheduled_actions` vector therefore grows linearly
with unresolved schedules and shrinks via a full scan per resolution.

## Adversarial Check

Incident analysis is operator-facing and runs on demand rather than in a hot
loop, so the constant factor is irrelevant for typical runs. But for
long-running workflows with hundreds of scheduled actions (e.g. a fan-out
workflow), the quadratic retain scan dominates the analysis time and the
allocation behavior is poor (retain rewrites the vector in place). A
`HashSet<ActionEvidenceKey>` would be O(1) per resolution with the same
semantics.

## Suggested Fix

Replace `Vec<SideEffectEvidence>` with a `BTreeSet` or `HashSet` keyed on
`ActionEvidenceKey`, or maintain a side-index `HashMap<ActionEvidenceKey, usize>`
if ordering must be preserved for diagnostics. Resolution then becomes O(1)
instead of O(N).
