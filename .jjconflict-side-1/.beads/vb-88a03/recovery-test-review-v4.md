# Recovery Test Review — vb-88a03 (v4)

**Reviewer:** RTX5090 independent TEST review
**Workspace:** /home/lewis/src/isoloated/vb-88a03-recovery-test-review-v4
**Source state:** READONLY — source not modified by reviewer
**Candidate:** b752db4c1a665caf90fa9e63e43d1505b0ba651d
**Base:** dba19b95918aca7c23e91b8814afa2a9118449f8
**Scope:** 16 manifest files, all read explicitly with bounded ranges

## Scope Summary

This bead adds six new integration tests and retains the original fourteen direct API acceptance tests plus one unit test, totaling twenty-one tests in scope.

**New bead tests (6):** Five genuine recovery integration cases plus one wrong-run ask cross-runtime forgery test.

**Retained base tests (15):** Fourteen original direct API acceptance tests plus one policy pin durable recovery test from the legacy binary.

**Files reviewed:** All sixteen manifest inputs read in full with explicit line ranges.

## New Recovery Integration Tests

### 1. vb_c4hzw_emergency_reopen.rs

**Scenario:** Emergency deny marker persists across journal close and reopen; a new legal live execution is still blocked at the Emergency layer with a complete nine-field decision record.

**Test architecture:** Three phases. Phase A submits a genuine artifact, registers the pack, and persists the accepted artifact to disk. Phase B triggers emergency deny activation via the runtime API, then drops all handles including the runtime, storage journal, and FjallJournal to force the emergency marker onto disk via Fjall's internal persistence. Phase C reopens the journal fresh, verifies the durable emergency state via direct Fjall inspection, creates a new runtime instance, configures a live Allowed policy, submits a new legal run, and attempts execution.

**Oracles and assertions:** An independent oracle module (oracle.rs) computes the expected decision record from the ticket, admission policy pin, live policy identity, and input value. The test asserts the complete nine-field DecisionRecord via structural equality: ticket, admission, current_live, input_digest, taint, outcome, layer, emergency_influence, and compat_influence. It also asserts the persisted PolicyDecisionEvent matches the expected record exactly, the ticket is unchanged, the adapter has zero effects, and the pending boundary is unchanged before and after.

**Quality assessment:** Strong. The test demonstrates genuine emergency marker persistence by forcing the marker onto disk via the Fjall persistence layer, then proving it survives process restart by re-reading it fresh. The independent oracle computation ensures the expected record is derived from the test inputs rather than copied from production output.

### 2. vb_c4hzw_corrupt_marker.rs

**Scenario:** Emergency deny marker corrupted with unsupported version causes precise typed failure, not silent default.

**Test architecture:** Three phases. Phase A writes a genuine accepted artifact. Phase B directly writes version 99 to the emergency_deny keyspace using raw Fjall database access, creating a corrupt marker that no production code would generate. Phase C reopens, submits a legal run, and executes — expecting the precise typed error.

**Oracles and assertions:** Uses Rust's `matches!` macro to verify the exact error variant chain: `RegisteredExecutionError::Runtime(RuntimeError::StorageJournalAppend { source })` where source is `JournalError::EmergencyDenyMarkerUnsupportedVersion { version: 99 }`. The version number 99 is asserted explicitly, not just the presence of an error. Zero effects, unchanged pending boundary, and absence of any PolicyDecisionEvent are also asserted.

**Quality assessment:** Strong. The test exercises the rare path where the emergency marker is corrupt rather than active or inactive. The version 99 literal makes the corruption unambiguous. The assertion structure ensures only the exact error variant passes.

### 3. vb_c4hzw_historical_replay.rs

**Scenario:** Historical Allowed decision under current Emergency deny; recovery preserves the historical state without re-executing.

**Test architecture:** Submits and executes a run to completion, then crashes at the PendingAction boundary by recording the durable receipt without completing the step. Reopens, activates current deny, and calls recover_product twice to prove idempotent recovery.

**Oracles and assertions:** Computes the expected historical Allowed decision independently. Asserts recovery product and summary are identical before and after current deny. Asserts full journal events are unchanged. Asserts pending boundary is unchanged. Asserts effect count remains 1 (historical receipt preserved, no re-execution). Asserts register value remains 100. Asserts stored decision matches the independent oracle exactly.

**Quality assessment:** Strong. The test captures the precise crash window between effect and completion, then proves recovery preserves the historical record under current deny. The idempotent recover_product calls prove deterministic recovery.

### 4. vb_c4hzw_legacy_admission/legacy_admission_test.rs

**Scenario:** Legacy Unverified admission recovered and subjected to new live effect under current Allowed policy.

**Test architecture:** Writes a genuine legacy RunAdmission event with RunPolicyPin::Unverified, StepStarted, and AskScheduledEvent. Reopens, recovers, answers the Ask, and attempts to execute the new Do step under current Allowed policy.

**Oracles and assertions:** An independent oracle computes the expected Denied decision. Asserts the decision outcome is Denied at the Admission layer. Asserts the admission field is PinnedAdmissionPolicy::Unverified. Asserts the ticket run and step match. Asserts the input digest matches the independent computation. Asserts the complete decision record matches the independent oracle. Asserts zero adapter effects, unchanged pending ticket, and exactly one PolicyDecisionEvent persisted.

**Quality assessment:** Strong. The test demonstrates that legacy Unverified admissions are correctly promoted to Pinned after recovery and that new live effects under those admissions are denied at the Admission layer when the current policy would have allowed them. The independent oracle ensures the expected record is not derived from production output.

### 5. vb_c4hzw_legacy_admission/forged_admission_test.rs

**Scenario:** Forged policy pin in legacy RunAdmission rejected by recovery before mutation.

**Test architecture:** Writes a legacy RunAdmission with a forged policy pin (wrong pin), StepStarted, and AskScheduledEvent. Attempts both recover_product and recover_and_resume, expecting rejection with the exact reason string.

**Oracles and assertions:** Computes the genuine policy digest from the artifact and constructs a distinct wrong pin (0xFF bytes). Asserts both recovery APIs reject with `RecoveryCannotResume { reason: "admission_policy_pin_mismatch" }`. Asserts full journal equality before and after both recovery attempts. Asserts full pending boundary equality. Asserts zero register effects after both attempts.

**Quality assessment:** Strong. The test verifies both recovery API entry points and proves no mutation occurs on rejection, including full journal and boundary equality assertions.

### 6. vb_vt2f_direct_runtime_api_acceptance/wrong_run_ask.rs

**Scenario:** Ask ticket issued by one runtime rejected when submitted to an unrelated runtime.

**Test architecture:** Creates two separate runtimes. Issuer runtime submits and suspends on an Ask, generating a genuine ticket. Receiver runtime submits and suspends on an unrelated Ask. Attempts to answer the issuer's ticket on the receiver runtime.

**Oracles and assertions:** Captures full state snapshots of both runtimes before the cross-runtime answer attempt. Asserts rejection with `RuntimeError::RunNotFound`. Asserts full journal equality for both runtimes. Asserts receiver snapshot, trace, active list, counters, and pending boundary are all unchanged. Asserts issuer snapshot, trace, active list, counters, and pending boundary are all unchanged. Asserts receiver ticket remains unchanged.

**Quality assessment:** Strong. The test proves AskTicket is not a cross-runtime forgery vector. The comprehensive state equality assertions before and after provide strong assurance of no mutation.

## Retained Original Tests

### vb_vt2f_direct_runtime_api_acceptance.rs

Fourteen original direct API acceptance tests verified by source hash against the trusted base. No new assertions added in this bead; original assertions retained. Tests cover submit, tick, fail, complete, answer, recover, and inspect API calls with various error paths.

### action_recovery_tests.rs

One unit test (recover_restores_action_contracts_schedules_completes_and_finishes) with extensive helper function (assert_raw_checkpoint_wrong_pin_rejected). Uses a mock artifact store and volatile runtime journal — explicitly unit test scope, not genuine Fjall-backed recovery. The helper function verifies raw 7-argument recovery produces Unverified admission, rejects forged policy pin with exact reason, rejects policy mismatch, and includes a positive control proving valid checkpoints recover successfully.

## Test Count Verification

- New bead tests: 6 (5 recovery integration + 1 wrong run ask)
- Retained direct API tests: 14
- Retained unit test: 1
- **Total in scope: 21 tests**

Additional recovery tests exist in the repository (recovery_integration.rs, recovery_bdd_tests.rs, recovery_hydration_tests.rs, policy_pin_durable_recovery.rs) but are not in this bead's scope per the manifest.

## Coverage Analysis

The bead covers the following recovery scenarios:

1. Emergency deny persistence and survival across restart
2. Corrupt emergency marker with unsupported version
3. Historical replay under current deny (idempotent recovery)
4. Legacy Unverified admission recovery and promotion
5. Forged policy pin rejection
6. Cross-runtime ask ticket rejection

The five genuine recovery integration cases are well-designed: each exercises a distinct recovery hazard, uses real Fjall-backed storage, demonstrates genuine crash/recovery semantics, and verifies both the positive path (recovery succeeds) and negative invariants (no mutation on failure, exact error variants).

## Verdict

All sixteen manifest files reviewed. Test design, assertion quality, and oracle independence meet acceptance criteria. The bead adds six new integration tests covering genuine recovery hazards and retains the original fourteen direct API acceptance tests.

**APPROVED**