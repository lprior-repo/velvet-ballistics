# SA-013: `idempotency_evidence` ownership is asymmetric between relaxed and checked admission paths

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_storage/src/admission/flow.rs:73-107`
- **Confidence**: confirmed

## Description

`submit_relaxed_artifact_with_evidence` borrows `&IdempotencyEvidence` and `.clone()`s the inner `Box<[ActionId]>`s. `submit_checked_artifact_with_evidence` takes `IdempotencyEvidence` by value and moves it. The two functions have identical responsibilities (build proof, attach evidence) but differ in ownership pattern for no apparent reason — both call sites receive the value from `submit_artifact_for_policy` which owns it.

## Evidence

```rust
// crates/vb_storage/src/admission/flow.rs:73-86 (relaxed)
fn submit_relaxed_artifact_with_evidence(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    required_capabilities: Box<[vb_core::capability::Capability]>,
    idempotency_evidence: &IdempotencyEvidence,                       // <-- borrow
    ...
) -> Result<AcceptedArtifact, JournalError> {
    ...
    proof.idempotency_keyed = idempotency_evidence.keyed.clone();     // <-- clone
    proof.idempotency_attested = idempotency_evidence.attested.clone();
    ...
}

// crates/vb_storage/src/admission/flow.rs:88-99 (checked)
fn submit_checked_artifact_with_evidence(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
    required_capabilities: Box<[vb_core::capability::Capability]>,
    idempotency_evidence: IdempotencyEvidence,                        // <-- by value
    ...
) -> Result<AcceptedArtifact, JournalError> {
    ...
    proof.idempotency_keyed = idempotency_evidence.keyed;             // <-- move
    proof.idempotency_attested = idempotency_evidence.attested;
    ...
}
```

The dispatch site `submit_artifact_for_policy` (line 50-71) already owns `admission_inputs: AdmissionInputs` by value, so it could move `idempotency_evidence` into either branch. The relaxed branch's `&IdempotencyEvidence` forces a clone that the caller did not need.

## Adversarial Check

The clone is on a `Box<[vb_core::ActionId]>` — `ActionId` is `Copy` (`u16` newtype in vb_core), so the clone is a single allocation of N·2 bytes. For typical contracts (a handful of actions), this is negligible. The defect is consistency/readability, not throughput. Functional-rust style (Holzman) calls for "same shape, same pattern" — having two sibling functions diverge on ownership is the kind of drift that produces bugs when one is later edited and the other is forgotten.

## Suggested Fix

Make both functions take `IdempotencyEvidence` by value and move the data:

```rust
fn submit_relaxed_artifact_with_evidence(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    required_capabilities: Box<[vb_core::capability::Capability]>,
    idempotency_evidence: IdempotencyEvidence,
) -> Result<AcceptedArtifact, JournalError> {
    ...
    proof.idempotency_keyed = idempotency_evidence.keyed;
    proof.idempotency_attested = idempotency_evidence.attested;
    ...
}
```

Update `submit_artifact_for_policy` to move `admission_inputs.idempotency_evidence` into the chosen branch.
