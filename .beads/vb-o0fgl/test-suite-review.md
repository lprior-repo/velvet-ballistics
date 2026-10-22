# Test Suite Review: vb-o0fgl

**Candidate:** vb-s4hfc (full typed DecisionRecord persistence)
**Commit:** 6de76b998c6b1f118b98d4bd1235619b34b252b7
**Main base:** 0434f64f0f1f4e2ab89ffd49628fcae903f809d3
**Reviewer workspace:** /home/lewis/src/isoloated/velvet-ballistics-vb-o0fgl-local3090-test-review
**Evidence prefix:** .beads/vb-o0fgl/evidence/TEST_REVIEW_V1_01, TEST_REVIEW_V1_02

## Verification Runs

| Command | Tests | Result | Evidence |
|---------|-------|--------|----------|
| `cargo test -p vb_storage --test policy_decision_full_record` | 15 | PASS | TEST_REVIEW_V1_01 |
| `cargo test -p vb_storage --lib` | 1617 | PASS | TEST_REVIEW_V1_02 |

Source hashes verified identical before and after test runs. No source mutation.

## Contract Coverage

| Contract requirement | Test | Verdict |
|---------------------|------|---------|
| All DecisionRecord fields survive Fjall strict append, close/reopen, events_for_run | `fjall_persistence_survives_reopen` | PASS |
| Round-trip equality for complete record including all enum variants | `policy_decision_event_round_trip`, `all_decision_enum_variants_round_trip` | PASS |
| Stable tag 38 assigned and used | `policy_decision_event_round_trip` (asserts id() == 38) | PASS |
| Old tags preserved | `old_tag_10_fixture_unchanged` | PASS |
| Malformed/truncated events rejected | `malformed_payload_rejected`, `truncated_payload_rejected` | PASS (see findings) |
| No journal-seq/ticket-seq conflation | `journal_seq_not_ticket_seq` | PASS |
| Run envelope mismatch protected | `run_envelope_mismatch_protected` (encode), `policy_decode_rejects_mismatched_run` (decode) | PASS |
| Terminal state preservation | `neutral_policy_fact_does_not_reset_lifecycle` | PASS |

## Full DecisionRecord Identity

`policy_decision_event_round_trip` verifies complete identity:
- All 8 ticket fields: run, step, seq, action, run_attempt, attempt, idempotency_key (computed canonically), capacity
- admission (PinnedAdmissionPolicy::Artifact with digest)
- current_live (CurrentPolicyIdentity::Configured with digest)
- input_digest
- taint (Taint::Secret)
- outcome (DecisionCode::Denied)
- layer (PolicyLayer::Emergency)
- emergency_influence (InfluenceState::Active)
- compat_influence (InfluenceState::NotEvaluated)

## Valid-Layout Positive Control

`policy_decode_rejects_mismatched_run` first successfully decodes a valid tag38 tuple with matching runs, asserts decoded fields, then tests the same typed layout with mismatched outer run. Positive control is present and concrete.

## Same-Run Sequence Ordering

`multiple_ordered_policy_events` verifies 3 events appended and read back in exact order.

## Mechanical Updates to Original Tests

Three original test files received mechanical updates compared to main:

1. **crates/vb_storage/src/codec/tests/replay_integrity.rs** — Updated kind 38 expectations: `is_known_record_kind(38)` now true, `validate_kind_family` admits 38 for journal magic, kind 39 remains unknown. No assertions deleted or weakened.

2. **crates/vb_storage/src/recovery/recovery_unit_tests.rs** — Added match arm `JournalEvent::PolicyDecisionEvent { .. } => Ok(false)` in lifecycle derivation test. Mechanical exhaustiveness fix.

3. **crates/vb_storage/src/recovery/tests.rs** — Added `| JournalEvent::PolicyDecisionEvent { .. }` to neutral event match arm. Mechanical exhaustiveness fix.

No historical test deleted or ignored. No assertion deletion or weakening in any original test.

## Findings

### Finding 1: Generic is_err() in malformed_payload_rejected (MINOR)

**File:** crates/vb_storage/tests/policy_decision_full_record.rs, lines 255-266

```rust
let result = decode_journal_event(&encoded, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
assert!(result.is_err(), "malformed payload should be rejected");
```

**Issue:** `is_err()` accepts ANY `JournalError` variant. The test would pass if decode returned `InvalidEvent`, `BadMagic`, or `Fjall` error instead of the expected `PostcardDecodeFailed`. Mutation that survives: swap error variant in malformed decode path.

**Repair:** Assert specific variant:
```rust
assert!(matches!(result, Err(JournalError::PostcardDecodeFailed(_))));
```

### Finding 2: Generic is_err() in truncated_payload_rejected (MINOR)

**File:** crates/vb_storage/tests/policy_decision_full_record.rs, lines 268-297

```rust
let result = decode_journal_event(&truncated, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
assert!(result.is_err(), "truncated payload should be rejected");
```

**Issue:** Same as Finding 1. Truncated envelope should yield `UnexpectedEof` or `HeaderLengthMismatch`, but any error passes.

**Repair:** Assert specific variant (expected `UnexpectedEof` or `HeaderLengthMismatch`):
```rust
assert!(matches!(result, Err(JournalError::UnexpectedEof)));
```

### Finding 3: Generic is_err() in envelope_kind_mismatch_rejected (MINOR)

**File:** crates/vb_storage/tests/policy_decision_full_record.rs, lines 317-327

```rust
assert!(result.is_err(), "envelope/payload kind mismatch should be rejected");
```

**Issue:** Should yield `RecordKindPayloadMismatch`, but any error passes.

**Repair:**
```rust
assert!(matches!(result, Err(JournalError::RecordKindPayloadMismatch { .. })));
```

### Finding 4: old_tag_10 test scope (OBSERVATION)

**File:** crates/vb_storage/tests/policy_decision_full_record.rs, lines 396-425

The test verifies tag byte 10 for RunAccepted and roundtrip equality. It does NOT verify a complete historical wire fixture with exact legacy bytes. This matches the coordinator's scope documentation which explicitly states "The tag10 test only checks tag+roundtrip, not a complete historical fixture."

## STATUS: REJECTED

Reason: Three generic `is_err()` assertions in malformed/truncated/mismatch tests could accept unrelated corruption errors. Per test-review skill: "is_ok(), is_err(), Some(_), and boolean smoke assertions are lethal unless the contract is explicitly boolean." The contract returns typed `JournalError`, so concrete variant assertions are required. All other acceptance criteria are met; this is a focused repair in the new test file only.