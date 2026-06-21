# SR-008: `reject_workflow_digest_mismatch` silently passes when no `RunAccepted` event exists

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_storage/src/recovery/replay/summary/frame_seed.rs:89`
- **Confidence**: confirmed

## Description

`reject_workflow_digest_mismatch` walks the event slice looking for the first
`RunAccepted`. If one is found with a divergent digest it returns
`CompiledIrDigestMismatch`; if one is found with a matching digest it returns
`Ok(())`; if no `RunAccepted` is present at all it falls through to
`map_or(Ok(()), ...)` and silently succeeds. The net effect is that a
journal without any `RunAccepted` event passes the "digest mismatch" gate.

## Evidence

```rust
pub fn reject_workflow_digest_mismatch(
    events: &[JournalEvent],
    expected: WorkflowDigest,
) -> RecoveryResult<()> {
    events
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunAccepted { workflow, .. } if *workflow != expected => {
                Some(Err(RecoveryError::CompiledIrDigestMismatch { ... }))
            }
            JournalEvent::RunAccepted { .. } => Some(Ok(())),
            _ => None,
        })
        .map_or(Ok(()), |result| result)         // <-- None => Ok(())
}
```

The caller is `recover_runtime_frame_seed_from_events_with_workflow`
(frame_seed.rs:79):
```rust
pub fn recover_runtime_frame_seed_from_events_with_workflow(
    events: &[JournalEvent],
    workflow: &vb_core::CompiledWorkflow,
) -> RecoveryResult<RecoveryFrameSeed> {
    reject_workflow_digest_mismatch(events, workflow.digest())?;
    recover_runtime_frame_seed_from_events_inner(events, Some(workflow))
}
```

If the events slice has no `RunAccepted` event (e.g. because the caller
passed a tail-only slice — see SR-001/SR-002 — or because the journal is
genuinely corrupt), the function returns `Ok(())` and recovery proceeds with
a workflow that may not match the persisted run.

## Adversarial Check

A plausible reading is "the function's name is `reject_..._mismatch`, so
returning Ok when there is nothing to compare is correct." But the only
caller uses the function as a *gate* before reconstructing live state from a
workflow that the caller asserts matches the run. Failing closed on missing
evidence is the safe behavior; silently succeeding on missing evidence
defeats the purpose of the gate. Compare `verify_run_admission_evidence`
(admission.rs:24) which explicitly returns `PolicyDigestExpectationMissing`
when no admission evidence is found — the analogous "missing RunAccepted"
case here should do the same.

## Suggested Fix

Return a typed error when `find_map` produces `None`:
```rust
pub fn reject_workflow_digest_mismatch(
    events: &[JournalEvent],
    expected: WorkflowDigest,
) -> RecoveryResult<()> {
    events
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunAccepted { workflow, .. } if *workflow != expected => {
                Some(Err(RecoveryError::CompiledIrDigestMismatch { expected, found: *workflow }))
            }
            JournalEvent::RunAccepted { .. } => Some(Ok(())),
            _ => None,
        })
        .unwrap_or_else(|| {
            // No RunAccepted evidence in the slice — fail closed rather than
            // silently letting recovery proceed with an unverified workflow.
            events.first().map_or(
                Err(RecoveryError::NoRecoveryData { run: RunId::new(0) }),
                |first| Err(RecoveryError::ReplayDivergence {
                    step: StepIdx::ZERO,
                    detail: format!(
                        "RunAccepted evidence missing for run {:?}; cannot verify workflow digest",
                        first.run_id()
                    ),
                }),
            )
        })
}
```
