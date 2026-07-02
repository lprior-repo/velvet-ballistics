# Test Repair Guide — vb-qi37.1.4

## Finding Summary

The test-plan and test-suite for vb-qi37.1.4 are REJECTED. The test-writer-report.md describes tests for vb-qi37.8 (gate validation pipeline), not vb-qi37.1.4 (runtime recovery / fail-closed boundary). The 9 INV-RC invariants in contract.md have zero explicit test coverage.

---

## Repair Step 1: Write test-plan.md for vb-qi37.1.4 Contract

Replace the current test-plan.md (which covers hot-loop engine behaviors 1-40) with scenarios covering the 9 recovery invariants.

### Required BDD Scenarios

**INV-RC-001**: `hydrate_run_frame` returns `Err(InvalidRecoveryHydration)` when `UnsupportedRecoveryState::slot_values` is `true`
- Scenario: Given a seed with `slot_values: true`, When `hydrate_run_frame()` is called, Then result is `Err(RuntimeError::InvalidRecoveryHydration)`

**INV-RC-002**: `hydrate_run_frame` returns `Err(InvalidRecoveryHydration)` when `UnsupportedRecoveryState::slot_taint` is `true`
- Scenario: Given a seed with `slot_taint: true`, When `hydrate_run_frame()` is called, Then result is `Err(RuntimeError::InvalidRecoveryHydration)`

**INV-RC-003**: `hydrate_run_frame` returns `Err(InvalidRecoveryHydration)` when `UnsupportedRecoveryState::action_payloads` is `true` ← PRIMARY GAP
- Scenario: Given a seed with `action_payloads: true`, When `hydrate_run_frame()` is called, Then result is `Err(RuntimeError::InvalidRecoveryHydration)`

**INV-RC-004**: `hydrate_run_frame` returns `Err(InvalidRecoveryHydration)` when `pending_actions` is nonempty and `unsupported.pending_actions` is `true`
- Scenario: Given a seed with `pending_actions: [Action {...}]` and `unsupported.pending_actions: true`, When `hydrate_run_frame()` is called, Then result is `Err(RuntimeError::InvalidRecoveryHydration)`

**INV-RC-005**: No action result body consumed from recovered frame when `action_payloads` is unsupported
- Scenario: Given a frame hydrated with `action_payloads: true`, When runtime attempts to read action result from frame, Then the runtime receives None/empty (exact error variant TBD based on API)

**INV-RC-006**: `DigestCheck::Full` verifies action ABI digest
- Scenario: Given a journal with action having ABI digest X, When `verify_digests(DigestCheck::Full, ..., found_abi_digest=Y)` is called with Y != X, Then result is `Err(RecoveryError::ActionAbiMismatch)`

**INV-RC-007**: `RunResumed`, `RunRetried`, `RunAnswered` events not silently dropped in `replay_events`
- Scenario: Given a journal with events `[RunAccepted, RunResumed, RunFinished]`, When `replay_events` is called, Then the returned sequence includes `RunResumed` event

**INV-RC-008**: `verify_digests` returns `ActionAbiMismatch` on digest mismatch
- Covered by INV-RC-006 scenario above

**INV-RC-009**: `verify_digests` returns `PolicyDigestMismatch` on policy digest mismatch
- Scenario: Given a journal with step having policy digest X, When `verify_digests(DigestCheck::Full, ..., found_policy_digest=Y)` is called with Y != X, Then result is `Err(RecoveryError::PolicyDigestMismatch)`

---

## Repair Step 2: Write Unit Tests in vb_runtime/recovery.rs

Add to the `#[cfg(test)]` module in `crates/vb_runtime/src/recovery.rs`:

```rust
#[test]
fn durable_frame_recovery_boundary_rejects_action_payloads_unsupported() {
    let summary = RecoveryRuntimeSummary {
        run: RunId::new(99),
        first_seq: EventSeq::new(0),
        last_seq: EventSeq::new(1),
        workflow: Some(WorkflowDigest::from_bytes([9; 32])),
        steps_started: 1,
        steps_succeeded: 1,
        actions_scheduled: 1,
        actions_resolved: 0,
        suspensions: 0,
        slots_written: 0,
        terminal: None,
    };
    let seed = RecoveryFrameSeed {
        summary,
        first_step: StepIdx::ZERO,
        step_count: 2,
        slot_count: 0,
        pc: StepIdx::ZERO,
        steps: vec![
            RecoveredStepEntry { step: StepIdx::ZERO, state: RecoveredStepState::Succeeded },
        ],
        slots: Vec::new(),
        pending_actions: vec![RecoveredActionEntry { /* ... */ }],
        unsupported: UnsupportedRecoveryState {
            slot_values: false,
            slot_taint: false,
            action_payloads: true,  // PRIMARY GAP: must trigger rejection
            pending_actions: false,
        },
    };
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);
    assert_eq!(
        boundary.hydrate_run_frame(),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
}
```

Similarly add tests for INV-RC-001, INV-RC-002, INV-RC-004.

---

## Repair Step 3: Write Digest Verification Tests

Add to `crates/vb_storage/src/recovery/tests.rs` or a new `vb_storage/recovery/digest_tests.rs`:

- Test `verify_digests(DigestCheck::Full, ...)` with mismatched action ABI digest → `ActionAbiMismatch`
- Test `verify_digests(DigestCheck::Full, ...)` with mismatched policy digest → `PolicyDigestMismatch`
- Test `verify_digests(DigestCheck::Full, ...)` with all digests matching → `Ok(())`

---

## Repair Step 4: Rewrite test-writer-report.md

Update the report to:
1. State it covers vb-qi37.1.4, not vb-qi37.8
2. Reference test files in `/home/lewis/src/vb-qi37-1-4/` workspace
3. Map each test to the INV-RC invariant it covers
4. Remove all references to gate G7-G15, gate_tests.rs, BDD gate scenarios

---

## Verification After Repair

After making these repairs, re-run test-reviewer from Tier 0. The reviewer will check:
1. test-plan-review.md shows APPROVED
2. test-suite-review.md shows APPROVED
3. All 9 INV-RC invariants have at least one BDD scenario with exact error assertions
4. All `action_payloads: true` rejection tests call `hydrate_run_frame()` (not just `summary()`)
