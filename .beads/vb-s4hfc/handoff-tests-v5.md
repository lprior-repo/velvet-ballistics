# VB-S4HFC Final Test Repair — Handoff

## Summary

All 15 tests in `tests/policy_decision_full_record.rs` now pass with real assertions, proper idempotency keys, and no unchecked operations.

## What Was Fixed

### A) Run Envelope Mismatch Protection

- `run_envelope_mismatch_protected`: Now captures the `encode_record` result and asserts `is_err()` instead of propagating the error with `?`. The serializer correctly rejects PolicyDecisionEvent when outer run != ticket.run.
- **New test** `policy_decode_rejects_mismatched_run`: Decode boundary attack test that constructs raw postcard bytes for a PolicyDecisionEvent with mismatched outer run (bypassing the serializer's check), then verifies the deserializer independently rejects it. This proves both encode and decode have independent run-envelope validation.

### B) Unchecked Operations Removed

- `i as u64` (2 occurrences): Replaced with `u64::try_from(i).map_err(...)?` for checked conversion.
- `encoded[..half]` slice: Replaced with `truncated.clone(); truncated.truncate(half)` using checked division.
- `seq += 1` unchecked arithmetic: Replaced with enumerated loop using checked `u64::try_from(i)` for seq values.
- `layers[i]` unchecked indexing: Replaced with `.zip()` for safe parallel iteration.

### C) Idempotency Key Fix

- `make_ticket()` now computes `idempotency_key` using `vb_core::action::compute_action_idempotency_key(run, run_attempt, seq, action)` instead of hardcoded `0`.
- Updated `policy_decision_event_round_trip` to assert the computed canonical key value.
- All ticket fields are distinct and meaningful: `run_attempt: 1`, `attempt: 1`, `capacity: 10`.

### D) Legacy Tag Test

- `old_tag_10_fixture_unchanged` retained as-is: verifies variant tag 10 is first byte and round-trips correctly. No complete historical fixture was derived from source (optional improvement deferred).

### E) Function Length and Style

- `all_outcome_layers_taints_round_trip` replaced 405-event combinatorial loop with 5 typed test cases covering all 5 taints, all 3 outcomes, all 3 layers, and all 3 influence states with full DecisionRecord equality.
- All functions ≤ 25 logical lines.
- No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`.
- No unchecked indexing, slicing, casts, or arithmetic.

## Test Results

```
running 15 tests
test distinct_nonzero_digests ... ok
test influence_state_values_distinct ... ok
test envelope_kind_mismatch_rejected ... ok
test all_decision_enum_variants_round_trip ... ok
test neutral_policy_fact_does_not_reset_lifecycle ... ok
test old_tag_10_fixture_unchanged ... ok
test malformed_payload_rejected ... ok
test journal_seq_not_ticket_seq ... ok
test policy_decode_rejects_mismatched_run ... ok
test run_envelope_mismatch_protected ... ok
test policy_decision_event_round_trip ... ok
test truncated_payload_rejected ... ok
test fjall_persistence_survives_reopen ... ok
test multiple_ordered_policy_events ... ok
test all_outcome_layers_taints_round_trip ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Exit code: 0 (via python gate script)
Evidence: `.beads/vb-s4hfc/evidence/FINAL_TEST_V5_UNIQUE`

## Known Limitations

1. The legacy tag 10 test only verifies the first byte and round-trip; no complete historical fixture was derived.
2. The decode attack test uses a known postcard wire format (newtype variant marker 0x05, variant index byte); if postcard changes its enum encoding, this test would need updating.

## Files Modified

- `crates/vb_storage/tests/policy_decision_full_record.rs` — all test fixes
- `crates/vb_storage/tests/policy_decision_full_record/pd_helpers.rs` — idempotency key computation
