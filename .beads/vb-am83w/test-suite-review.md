# Test Suite Review: vb-am83w (vb-s4hfc codec child)

Role: LOCAL3090 TEST REVIEWER (fresh, independent)
Bead: vb-am83w
Candidate: 7a5586b2f
Base: 5fff342a7
Date: 2026-10-22

## Scope
Review ONLY seven new tests in `crates/vb_storage/tests/policy_decision_wire.rs` for CODEC-only child bead vb-s4hfc. Parent prototype known unaccepted; full ticket/admission/identity/dispatch/authorization/replay remains open on parent. This review does NOT approve full parent.

## Source/Test State
- All source and test files strictly read-only during review.
- Nine-file frozen manifest verified via SHA256 before and after test execution. No changes detected.
- No tests added, mutants run, formatting applied, or repairs attempted.

## Test Execution
Command: `python3 /tmp/vb-run-gate.py 300 .beads/vb-am83w/evidence/TEST-AM83W-FRESH cargo test -p vb_storage --test policy_decision_wire`
- Exit code: 0
- Duration: 10.561s
- Tests run: 7
- Passed: 7
- Failed: 0
- Ignored: 0
- Filtered: 0

Evidence: `.beads/vb-am83w/evidence/TEST-AM83W-FRESH.{json,stdout,stderr}`

## Test-by-Test Analysis

### 1. test_policy_layers_all
Covers: All 4 PolicyLayer values (Admission, Replay, LiveEffect, EmergencyDeny).
Pattern: Serialize -> Deserialize -> assert_eq!(decoded, event).
Finding: Correct. Uses distinct run/seq/policy_digest/input_digest per test. No panic/unwrap/expect.

### 2. test_decision_codes_all
Covers: All 3 DecisionCode values (Allowed, Denied, Suspended).
Pattern: Serialize -> Deserialize -> assert_eq!.
Finding: Correct. Uses consistent PolicyLayer::Admission, varies only code.

### 3. test_flag_permutations
Covers: All 4 flag pairs (emergency_deny_active, compatibility_influence).
Pattern: Serialize -> Deserialize -> assert_eq! for each combination.
Finding: Correct. Exhaustive coverage of boolean flag space.

### 4. test_fjall_journal_round_trip
Covers: Full record equality via actual FjallJournal API.
Uses: append_strict, drop, reopen, events_for_run.
Finding: Correct. Uses distinct non-zero policy_digest (7) and input_digest (8). Verifies full record round-trip through actual journal machinery, not just in-memory serialization.

### 5. test_multiple_events_distinct_digests
Covers: Multiple events in same run preserve distinct digests.
Uses: EventSeq::new(0) and EventSeq::new(1) with distinct policy/input digests (9,10,11,12).
Finding: Correct. Ordered seq 0/1 with all different digest values. Proves no cross-event data corruption or reuse.

### 6. test_malformed_payload
Covers: Truncated payload returns error, not panic or garbage.
Pattern: Serialize valid event, truncate 4 bytes, assert!(result.is_err()).
Finding: Correct. Uses checked_sub and safe get for truncation (no panic on short wire). Asserts error state only, which is appropriate for malformed input.

### 7. test_legacy_tag_stability
Covers: RunAccepted still uses established tag 10.
Pattern: Serialize RunAccepted event, assert_eq!(wire.first().copied(), Some(10)).
Finding: Correct. Verifies wire tag stability for existing event type. Also verifies decode round-trip.

## Coverage Assessment

### Covered
- All 4 PolicyLayer values
- All 3 DecisionCode values
- All 4 flag pairs (emergency_deny_active, compatibility_influence)
- Full record equality (all fields + variants) via assert_eq!
- Actual FjallJournal API: append_strict, drop, reopen, events_for_run
- Nonzero distinct digests with ordered seq 0/1
- Malformed truncated payload -> is_err
- Established RunAccepted tag 10 fixture
- Codec routing and wire field regression detection

### Not Covered (Parent Scope, Expected)
- Ticket issuance and validation
- Admission identity binding
- Dispatch authorization
- Replay detection and enforcement
- Full policy lifecycle (evaluate -> record -> enforce)
- Authorization decision correctness (tests verify serialization, not policy logic)

These exclusions are intentional: vb-s4hfc is CODEC-ONLY. The parent bead retains these as open obligations.

## Quality Properties

### Panic/Unwrap/Expect
None found in test file. All error propagation via Result<T, E> and ? operator. Malformed test uses checked_sub and safe get.

### Determinism
Tests use fixed byte values for digests (no randomness). No sleeps, shared mutable state, or parallelism. TempDir cleanup via drop. Deterministic and reproducible.

### Public API
All tests use public API only: RunId::new, EventSeq::new, JournalEvent variants, postcard serialization, FjallJournal::open/append_strict/events_for_run.

### Mutation Resistance
- Removing any layer from the enum would fail test_policy_layers_all.
- Removing any code would fail test_decision_codes_all.
- Changing flag behavior would fail test_flag_permutations.
- Altering serialization layout would fail round-trip tests.
- Changing RunAccepted tag would fail test_legacy_tag_stability.
- Truncation handling change would fail test_malformed_payload.

## Verdict

APPROVED_FOR_SCOPED_CODEC_TESTS

The seven tests comprehensively cover the codec serialization/deserialization contract for the new PolicyDecisionEvent type and its interaction with the existing FjallJournal machinery. They detect wire format regressions, tag stability violations, field corruption, and malformed input handling. Tests are deterministic, use public API only, have no panic/unwrap/expect, and are resistant to implementation mutations.

Coverage gaps (ticket, identity, authorization, replay, dispatch) are explicitly outside this child bead's scope and remain open on the parent bead.
