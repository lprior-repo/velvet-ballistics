# vb-s4hfc Final Cleanup V7 — Handoff

## Summary
Three compatibility fixes applied to close exhaustive match errors and correct a weak test fixture. Both targeted test suites pass with 0 failures.

## Changes Applied

### 1. Exhaustive Match Fixes (compile errors from new variant)
- **`crates/vb_storage/src/recovery/tests.rs`** (line ~1111): Added `JournalEvent::PolicyDecisionEvent { .. }` to the neutral-event match arm in `summarize_events`. This is a non-accounting fact that does not affect the recovery summary counters.
- **`crates/vb_storage/src/recovery/recovery_unit_tests.rs`** (line ~1218): Added `JournalEvent::PolicyDecisionEvent { .. } => Ok(false)` to the non-accounting event arm in the VB-NOORE exhaustiveness checker. Consistent with the event's non-accounting semantics.

### 2. Test Fixture Correction (identity field coverage)
- **`crates/vb_storage/tests/policy_decision_full_record/pd_helpers.rs`**: Changed `run_attempt` from 1 to 2 and `attempt` from 1 to 3 in `make_ticket`. Recomputed the canonical idempotency key using the same `run_attempt=2` value to preserve record integrity.
- **`crates/vb_storage/tests/policy_decision_full_record.rs`**: Updated the matching assertions to expect `run_attempt=2`, `attempt=3`, and to compute the expected idempotency key with `run_attempt=2`.

### 3. Outdated Kind Admission Test
- **`crates/vb_storage/src/codec/tests/replay_integrity.rs`**: The `kind_28_29_31_through_35_admission_for_journal_magic` test was checking that kind 38 was unknown. With the addition of `PolicyDecisionEvent` (kind 38), this assertion was obsolete. Updated to verify kind 38 is known and admitted, and moved the unknown-kind boundary check to kind 39.

## Test Results

### Integration Suite (CLEANUP_V7_01)
```
running 15 tests
test distinct_nonzero_digests ... ok
test influence_state_values_distinct ... ok
test neutral_policy_fact_does_not_reset_lifecycle ... ok
test malformed_payload_rejected ... ok
test envelope_kind_mismatch_rejected ... ok
test all_decision_enum_variants_round_trip ... ok
test run_envelope_mismatch_protected ... ok
test old_tag_10_fixture_unchanged ... ok
test policy_decode_rejects_mismatched_run ... ok
test journal_seq_not_ticket_seq ... ok
test policy_decision_event_round_trip ... ok
test truncated_payload_rejected ... ok
test all_outcome_layers_taints_round_trip ... ok
test fjall_persistence_survives_reopen ... ok
test multiple_ordered_policy_events ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Library Tests (CLEANUP_V7_03)
```
test result: ok. 1617 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Evidence Files
- `.beads/vb-s4hfc/evidence/CLEANUP_V7_01.json` — integration test gate result
- `.beads/vb-s4hfc/evidence/CLEANUP_V7_01.stdout` — integration test output
- `.beads/vb-s4hfc/evidence/CLEANUP_V7_02.json` — first library test run (failed, exposed kind 38 issue)
- `.beads/vb-s4hfc/evidence/CLEANUP_V7_03.json` — second library test run (all passed)
- `.beads/vb-s4hfc/evidence/CLEANUP_V7_03.stdout` — library test output

## Modified Files (this cleanup)
1. `crates/vb_storage/src/recovery/tests.rs`
2. `crates/vb_storage/src/recovery/recovery_unit_tests.rs`
3. `crates/vb_storage/tests/policy_decision_full_record/pd_helpers.rs`
4. `crates/vb_storage/tests/policy_decision_full_record.rs`
5. `crates/vb_storage/src/codec/tests/replay_integrity.rs`

No production implementation changes were made. All changes are mechanical exhaustive-match updates or test fixture corrections. All 15 integration test cases and their assertions are preserved.
