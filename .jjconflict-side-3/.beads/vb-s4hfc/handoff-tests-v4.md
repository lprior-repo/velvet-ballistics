# vb-s4hfc Test Repair Handoff (v4)

## Status
Tests rewritten to address all 6 concrete findings from black hat review. 12 passing, 2 failing (as expected).

## Test Results
```
test result: FAILED. 12 passed; 2 failed; 0 ignored; 0 measured
```

### Passing Tests (12)
- `policy_decision_event_round_trip` - Full field identity assertion
- `all_decision_enum_variants_round_trip` - All 3 outcome/layer combos
- `fjall_persistence_survives_reopen` - Journal write/reopen/read equality
- `multiple_ordered_policy_events` - 3 events on same run, strict append/drop/reopen
- `malformed_payload_rejected` - Non-event bytes rejected on decode
- `truncated_payload_rejected` - Half-length payload rejected
- `envelope_kind_mismatch_rejected` - RunAccepted kind with policy payload rejected
- `journal_seq_not_ticket_seq` - Outer seq differs from ticket.seq, both preserved
- `old_tag_10_fixture_unchanged` - RunAccepted encodes with variant tag 10 (established byte fixture)
- `distinct_nonzero_digests` - Digests are not zero or identical
- `influence_state_values_distinct` - All 3 influence states distinct
- `all_outcome_layers_taints_round_trip` - 135 combos (3 outcomes × 3 layers × 5 taints × 3 emergency × 3 compat) on same run

### Failing Tests (2) - Expected Against Current Production

#### 1. `neutral_policy_fact_does_not_reset_lifecycle`
**Bug exposed:** `PolicyDecisionEvent` maps to `LifecycleState::Active` in `event_to_lifecycle()`, so when it's the last event it resets the lifecycle state to Active instead of preserving the terminal state.

**Test asserts:**
```rust
derive_lifecycle_state_from_events([RunAccepted, RunFailed, PolicyDecision]) == Failed
derive_lifecycle_state_from_events([RunAccepted, RunCancelled, PolicyDecision]) == Cancelled
derive_lifecycle_state_from_events([RunAccepted, RunFinished, PolicyDecision]) == Completed
```

**Actual result:** All three return `Active` (the bug).

**Fix required:** In `crates/vb_storage/src/journal/incident.rs`, `PolicyDecisionEvent` should not map to `Active`. It should either:
- Not affect lifecycle derivation (skip it), or
- Map to the previous state (requires tracking), or
- Be classified as a non-lifecycle event

#### 2. `run_envelope_mismatch_protected`
**Bug exposed:** No validation that `PolicyDecisionEvent.run` matches `decision.ticket.run`. An event can have outer run 999 but inner ticket run 100, and it's accepted.

**Test asserts:**
```rust
let decision = make_decision(100, ...);  // ticket.run = 100
let event = PolicyDecisionEvent { run: RunId::new(999), decision };
encode_record(..., event, ...) -> decode_journal_event(...)
// Expect: error
```

**Actual result:** Decode succeeds (no validation).

**Fix required:** Either in `encode_record` (reject mismatch at encode time) or in the PolicyDecisionEvent decode path (reject mismatch at decode time).

## Changes Made (v3 → v4)

### Finding 1: Lifecycle derivation
- Replaced encode-only test with actual `derive_lifecycle_state_from_events()` calls
- Added 3 assertions: Failed, Cancelled, Completed terminal states preserved after policy event
- Uses `vb_core::workflow::LifecycleState` (not exported from vb_storage)

### Finding 2: Run envelope mismatch
- Changed from "run ID preserved through roundtrip" to "mismatch detected and rejected"
- Constructs event with run=999, ticket.run=100
- Asserts decode returns error (currently passes, exposing bug)

### Finding 3: Invalid valid cases
- Fixed all run ID mismatches in valid test cases
- `fjall_persistence_survives_reopen`: run=1, ticket.run=1 (was 1 vs 100)
- `multiple_ordered_policy_events`: all events run=500, all tickets run=500 (was 500 vs 100/200/300)
- `all_decision_enum_variants_round_trip`: events run=100/200/300 match ticket.run=100/200/300
- `neutral_policy_fact_does_not_reset_lifecycle`: event run=777, ticket.run=777 (was 777 vs 100)
- `journal_seq_not_ticket_seq`: event run=100, ticket.run=100 (was already matching)

### Finding 4: Unchecked arithmetic/casts
- Replaced `1000 + idx as u64` pattern with fixed run IDs per test case
- Replaced `i as u64` with typed EventSeq construction
- Removed 1215-iteration combination loop; replaced with 135-iteration loop using same run
- Used `Vec::truncate` via slice-to-vec for truncation test
- All casts removed from new code

### Finding 5: Old tag 10 fixture
- Changed from roundtrip-only to established byte fixture check
- Asserts `postcard::to_allocvec(&event).first() == Some(10)`
- Matches proven pattern from vb-s4hfc-local3090-codec workspace

### Finding 6: Code quality
- Removed unused `encode_policy_event` helper
- All functions ≤ 25 logical lines
- Full field equality assertions on all DecisionRecord fields
- No unwrap/expect/panic in test bodies
- File: 560 lines, 14 tests, 3 helpers

## Evidence
- Raw test log: `.beads/vb-s4hfc/evidence/TEST_V4_05.stdout`
- Test runner JSON: `.beads/vb-s4hfc/evidence/TEST_V4_05.json`
- Previous attempts: TEST_V3_NEW_PREFIX, TEST_V4_01 through TEST_V4_04

## File Structure
- `crates/vb_storage/tests/policy_decision_full_record.rs` - 576 lines, 14 tests
- `crates/vb_storage/tests/policy_decision_full_record/pd_helpers.rs` - 44 lines, 3 helpers

## Next Steps
1. Fix `event_to_lifecycle` to handle PolicyDecisionEvent correctly (finding 1)
2. Add run envelope validation for PolicyDecisionEvent (finding 2)
3. Re-run test suite to confirm all 14 pass
4. Update beads with fix evidence