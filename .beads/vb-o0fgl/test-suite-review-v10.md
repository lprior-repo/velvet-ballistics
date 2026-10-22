# TEST Suite Review — vb-o0fgl (v10)

**Bead:** vb-o0fgl
**Candidate:** af97dc3217486887b89ff036ac6a4bbaf4f47120
**Base:** 0434f64f0f1f4e2ab89ffd49628fcae903f809d3
**Reviewer workspace:** /home/lewis/src/isoloated/velvet-ballistics-vb-o0fgl-local3090-final-v10
**Git root verified:** /home/lewis/src/isoloated/velvet-ballistics-vb-o0fgl-local3090-final-v10
**JJ root verified:** /home/lewis/src/isoloated/velvet-ballistics-vb-o0fgl-local3090-final-v10

## Scope

This is an independent TEST suite review only. No source, test, or config edits. No production CODE review claims. No Runtime live authorization/admission/deny persistence claims.

Review covers:
- 15 new tests in `crates/vb_storage/tests/policy_decision_full_record.rs`
- 1 new test helper in `crates/vb_storage/tests/policy_decision_full_record/pd_helpers.rs`
- 3 mechanically changed original test files (match arm additions and formatting for PolicyDecisionEvent variant)
- Contract parity against `.beads/vb-s4hfc/full-record-contract.md`
- Exact error path assertions per review brief

## V10 vs V9 Changes

V10 is V9 (c74c3102ba) plus mechanical formatting fixes applied with the pinned canonical formatter `rustup run nightly-2026-04-28 cargo fmt --all`. The actual diff across Rust paths is exactly 3 files:

1. `crates/vb_storage/src/codec/tests/replay_integrity.rs` — Long assert line wrapped across multiple lines. No assertion removed or weakened.
2. `crates/vb_storage/tests/policy_decision_full_record.rs` — `use vb_storage::JournalError;` moved from bottom of imports to alphabetical position.
3. `crates/vb_storage/tests/policy_decision_full_record/pd_helpers.rs` — `use vb_core::action::{compute_action_idempotency_key, ActionTicket};` reordered to `{ActionTicket, compute_action_idempotency_key}` for alphabetical sort.

No function body, behavior, or test logic changed in the V9→V10 diff. Production source is identical to V9.

## Source Manifest Verification

All 17 source file SHA-256 hashes verified against `.beads/vb-s4hfc/evidence/final-source-v10.json`. All match.

## Test Execution Evidence

### FINAL_V10_01: New Full Record Tests
```
python3 /tmp/vb-run-gate.py 600 .beads/vb-o0fgl/evidence/FINAL_V10_01 cargo test -p vb_storage --test policy_decision_full_record
exit_code: 0, seconds: 12.664
Result: running 15 tests — all passed; 0 failed; 0 ignored
```

### FINAL_V10_02: Library Tests
```
python3 /tmp/vb-run-gate.py 600 .beads/vb-o0fgl/evidence/FINAL_V10_02 cargo test -p vb_storage --lib
exit_code: 0, seconds: 11.584
Result: running 1617 tests — all passed; 0 failed; 0 ignored
```

### FINAL_V10_03: Canonical Format Check
```
python3 /tmp/vb-run-gate.py 180 .beads/vb-o0fgl/evidence/FINAL_V10_03 rustup run nightly-2026-04-28 cargo fmt --all --check
exit_code: 0, seconds: 1.527
Result: clean — no formatting issues
```

## Contract Coverage

| Contract Requirement | Test(s) | Status |
|---|---|---|
| All DecisionRecord fields survive Fjall strict append, close/reopen, events_for_run | fjall_persistence_survives_reopen, all_outcome_layers_taints_round_trip, multiple_ordered_policy_events | ✓ |
| Round-trip equality for complete record including all enum variants | policy_decision_event_round_trip, all_decision_enum_variants_round_trip, all_outcome_layers_taints_round_trip | ✓ |
| Stable tag 38 assigned and used | policy_decision_event_round_trip (asserts kind 38) | ✓ |
| Old tags preserved | old_tag_10_fixture_unchanged (asserts tag 10) | ✓ |
| Malformed/truncated events rejected | malformed_payload_rejected, truncated_payload_rejected | ✓ |
| No journal-seq/ticket-seq conflation | journal_seq_not_ticket_seq | ✓ |
| Run envelope mismatch protected | run_envelope_mismatch_protected, policy_decode_rejects_mismatched_run | ✓ |
| Policy fact does not reset lifecycle | neutral_policy_fact_does_not_reset_lifecycle | ✓ |

## Enum Variant Coverage

| Enum | Variants | All Covered |
|---|---|---|
| DecisionCode | Denied, Allowed, Suspended (3) | ✓ |
| PolicyLayer | Emergency, Live, Admission (3) | ✓ |
| InfluenceState | NotEvaluated, Active, Inactive (3) | ✓ |
| Taint | Clean, Secret, DerivedFromSecret, Random, TimeDependent (5) | ✓ |

## Exact Error Path Verification

| Error Path | Test | Assertion | Verified |
|---|---|---|---|
| malformed → PostcardDecodeFailed(SerdeDeCustom) | malformed_payload_rejected | Exact variant match | ✓ |
| truncation → UnexpectedEof | truncated_payload_rejected | Exact variant match | ✓ |
| kind mismatch → RecordKindPayloadMismatch{10,38} | envelope_kind_mismatch_rejected | Exact fields | ✓ |
| encode → Encode(SerdeSerCustom) | run_envelope_mismatch_protected | Exact variant match | ✓ |
| forged decode → SerdeDeCustom | policy_decode_rejects_mismatched_run | Exact variant match, after positive FULL equality decode control | ✓ |

All negative error paths assert exact Postcard/Serde error variants with precise field values where applicable. No generic `is_err()` or `is_ok()` smoke assertions for error paths.

## Valid FULL Equality Decode Control

`policy_decode_rejects_mismatched_run` correctly implements the required decode boundary test:
1. **Positive control:** valid tuple with matching runs encodes to tag 38 wire format, decodes back, and full decoded event is compared for equality against expected full record (`assert_eq!(decoded, expected)`).
2. **Negative control:** same layout with mismatched outer run (999 vs 100) encodes to tag 38 wire format, then decoding is expected to reject with exact `SerdeDeCustom` error.

This satisfies the contract requirement that the run-binding invariant is enforced at decode time, with a valid FULL equality decode control before the negative rejection.

## Mechanical Changes to Original Tests

1. `crates/vb_storage/src/codec/tests/replay_integrity.rs` — Updated known/unknown record kind expectations: kind 38 now known, kind 39 unknown. Also received mechanical formatting fix (assert wrapping). No weakening.
2. `crates/vb_storage/src/recovery/recovery_unit_tests.rs` — Added match arm for PolicyDecisionEvent. No assertion changes.
3. `crates/vb_storage/src/recovery/tests.rs` — Added match arm for PolicyDecisionEvent. No assertion changes.

No original tests deleted or ignored. No assertions weakened.

## Test Scope Accuracy

- `old_tag_10_fixture_unchanged` verifies tag 10 first byte and roundtrip equality only. This is accurate scope: old tags unchanged, not complete historical wire coverage. The storage library already contains original codec fixture tests for historical compatibility.
- No claim of complete historical schema coverage. Requirement is old tags unchanged.
- No Runtime policy/admission/control/fullv1 acceptance. Test tag 10 scope remains firstbyte+roundtrip.

## Findings

### V9 Formatting Findings — Fixed with Evidence

The V9 review found 3 formatting issues that failed `cargo fmt --all --check`. These are now fixed with evidence from FINAL_V10_03:

1. **replay_integrity.rs line 232 long assert line** — FIXED. Wrapped across multiple lines by canonical formatter. Evidence: FINAL_V10_03 exit 0, diff inspection confirms wrapping.
2. **pd_helpers.rs import order** — FIXED. `ActionTicket` now precedes `compute_action_idempotency_key`. Evidence: FINAL_V10_03 exit 0, diff inspection confirms reorder.
3. **policy_decision_full_record.rs JournalError import placement** — FIXED. Moved to alphabetical position among vb_storage imports. Evidence: FINAL_V10_03 exit 0, diff inspection confirms reorder.

Disposition for all three: `fixed_with_evidence` — actual final source inspected, diff confirmed mechanical formatting-only change, and canonical format gate FINAL_V10_03 passed.

### No BLOCKER or MAJOR findings

- All 15 new tests compile, execute deterministically, and pass
- All 1617 library tests pass
- All enum variants covered
- All specific error paths asserted exactly
- No generic negative error assertions
- Valid FULL equality decode control present
- No ignored tests, sleeps, or hidden shared mutable state
- Mutation resistance: deleting any branch/error logic would be caught by a named test with exact assertion
- No implementation detail assertions; tests use public API only
- Canonical format check passes

## Status

**STATUS: APPROVED**

All contract requirements covered. All specific error paths verified with exact Postcard/Serde variant assertions. All enum variants covered. All tests deterministic and passing. Canonical format check passes with evidence. V9 formatting findings are fixed with evidence tied to actual final source.

**Evidence location:** `.beads/vb-o0fgl/evidence/FINAL_V10_01.*`, `FINAL_V10_02.*`, `FINAL_V10_03.*`
