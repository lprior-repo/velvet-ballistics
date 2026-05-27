# Test Plan Review — vb-om21 State 10

reviewer_skill: test-reviewer
reviewer_invocation_id: test-reviewer-vb-om21-state10-001
bead_id: vb-om21
state: 10
sublane: test-plan-review
isolated_workdir: /home/lewis/isolated/femdation-velvet-ballistics/vb-om21
reviewed_at_utc: 2026-05-27T23:30:00Z
parent_invocation_id: test-writer-vb-om21-state9-001
reviewed_artifact: test-plan.md
supplementary_artifacts: contract.md, proof-to-rust-map.md, proof-to-rust-review.md
bead_classification: TEST-FIRST (production code deferred to State 11)

## Executive Summary

The test plan (`test-plan.md`) maps 11 behavior test functions to 8 requirements and 6 contract clauses, with traceability to all 52 proof obligations from the approved State 6/7 bridge. The plan correctly identifies open questions about the API surface, metadata injection mechanism, and error type placement. The plan is coherent with the proof-to-rust bridge and provides viable infrastructure guidance for the test-first approach.

**Verdict:** APPROVED — no blocking gaps in plan coverage.

## Plan Review Gates

### Gate 1: Every public behavior in contract.md has at least one GWT scenario

| Contract Clause | Test Functions | Status |
|---|---|---|
| C-vb-om21-prefix-bound | test_tail_scan_prefix_bound (5 variants), test_bounded_scan (4 variants) | COVERED |
| C-vb-om21-big-endian-max | test_big_endian_max_seq (5 variants), test_key_parse_no_panic (7 variants) | COVERED |
| C-vb-om21-tail-definition | test_zero_tail_empty_journal (4 variants), test_single_event_tail (4 variants), test_tail_overflow (4 variants) | COVERED |
| C-vb-om21-metadata-validation | test_tail_mismatch_rejection (5 variants), test_typed_error_distinction (5 variants) | COVERED |
| C-vb-om21-missing-journal | test_missing_journal_recovery (4 variants) | COVERED |
| C-vb-om21-replay-integrity | test_replay_parity (5 variants) | COVERED |

All 8 requirement IDs map to at least one test function. All 6 contract clauses are covered.

### Gate 2: Every error variant has a scenario asserting exact variant and fields

| Error Variant | Test Functions | Field Assertions |
|---|---|---|
| SequenceGap | test_tail_mismatch_rejection (variant 2), test_replay_parity (variant 2) | expected seq, actual seq verified |
| SequenceOverflow | test_tail_overflow (all variants) | overflow detection via checked_add |
| WrongRun | test_replay_parity (variants 3, 4) | expected/actual run_id verified (deferred sub-test noted) |
| TooManyEvents | test_bounded_scan (variant 1) | run, limit, observed fields verified |
| MissingJournal (planned) | test_missing_journal_recovery (all 4 variants) | field assertions deferred to State 11 |
| TailMismatch (planned) | test_tail_mismatch_rejection (variants 1-5) | field assertions deferred to State 11 |
| TailOverflow (planned) | test_tail_overflow (all variants) | field assertions deferred to State 11 |

**Finding F-VB-OM21-PLAN-001 (MEDIUM):** MissingJournal and TailMismatch are planned error variants that do not yet exist in the production error enum. The plan correctly notes they must be added at State 11. Tests targeting these variants can only assert on current public API behavior (which returns Ok(empty) for missing journal, and passes-through replay for metadata match). This is an honest API gap, not a plan defect.

### Gate 3: Assertions are concrete

The plan specifies concrete assertion patterns:
- `assert_eq!(events.len(), N)` for event counts
- `assert_eq!(event.seq().get(), i)` for sequence values
- Pattern match on `JournalError::SequenceGap { expected, actual }` with field value checks
- `prop_assert_eq!` and `prop_assert!` for property tests
- No `is_ok()`, `is_err()`, `Some(_)` boolean smoke assertions in the plan design

### Gate 4: Boundary cases are named

The plan explicitly lists boundary cases for each test function:
- Key length boundaries: 0, 1, 9, 17 bytes (test_key_parse_no_panic)
- Sequence boundaries: 0, 1, 255, u64::MAX-1, u64::MAX
- Run boundaries: empty, single, multiple runs
- Prefix boundaries: starts_with check, different run_id ordering
- Tail boundaries: 0, 1, u64::MAX-1, u64::MAX

### Gate 5: Non-trivial pure behavior has property tests planned

The plan specifies property tests for:
- Big-endian byte ordering (u64 pairs)
- Key encoding roundtrips (run_id, seq)
- Key length invariants (always 17 bytes)
- Prefix uniqueness across runs
- Same-run key differentiation

These are correctly scoped to `proptest!` macros.

### Gate 6: No parser/codec/hostile input fuzz requirement in contract scope

None of the 8 requirements involve parsing external input. The tail scan operates on internally-generated keys. Fuzz is correctly not mandated by this bead.

### Gate 7: Verifier harnesses are not counted as behavior tests

The plan correctly distinguishes behavior tests (workspace_tests) from refinement harnesses (kani_vb_om21_*.rs, verification/verus/*.rs, etc.). No overlap or double-counting.

### Gate 8: Proof-to-implementation rows covered by executable behavior tests

All 52 proof obligations from `proof-to-rust-map.md` are mapped to planned behavior test functions. The bridge correctly identifies `mapping_status: planned` for all rows.

## Plan Infrastructure Review

### Target File Registration

The plan correctly identifies the need to register the test target in `crates/workspace_tests/Cargo.toml`. Verified: the `[[test]]` entry exists at the claimed location.

### Helper Function Design

The plan's suggested helpers (`open_test_journal`, `seed_events`, `seed_event`, `run_id`) are well-scoped and match patterns in existing workspace tests.

### Open Questions Resolution

The plan's 5 open questions are honestly documented:

| OQ | Resolution | Status |
|---|---|---|
| OQ-1: API surface for tail query | `events_for_run` is public and suitable for integration tests | RESOLVED by test-writer |
| OQ-2: Metadata injection mechanism | No tail metadata field exists; tests operate on keyspace state directly | DOCUMENTED GAP — State 11 must add this |
| OQ-3: Error type placement | Tests use existing `JournalError` variants; new variants deferred | DOCUMENTED GAP |
| OQ-4: FjallJournal construction | Verified: `FjallJournal::open(dir, None)` pattern works | RESOLVED |
| OQ-5: Concurrent run isolation | Implicitly handled by `events_for_run` which holds a snapshot | RESOLVED by implementation |

## Lethal Finding Check

| Lethal Pattern | Status |
|---|---|
| Missing GWT scenarios per contract clause | CLEAR — all 6 clauses covered |
| Missing error variant tests | ACCEPTED — MissingJournal/TailMismatch are planned, not yet in production |
| Boolean-only assertions in plan | CLEAR |
| Unnamed boundary cases | CLEAR |
| Missing property tests for pure behavior | CLEAR |
| Verifier harnesses counted as behavior tests | CLEAR |
| Unbounded test commands | CLEAR — `cargo test` with exact target specified |

## Summary

| Metric | Value |
|---|---|
| Contract clauses covered | 6/6 (100%) |
| Requirements covered | 8/8 (100%) |
| Planned test functions | 11 |
| Planned test variants | 50+ |
| Property test functions planned | 6 |
| Open questions resolved | 3/5 (3 resolved, 2 deferred to State 11) |
| Blocking findings | 0 |
| Non-blocking findings | 1 (MEDIUM: MissingJournal/TailMismatch deferred) |

## Verdict

APPROVED. The test plan covers all contract clauses and requirements with concrete, sharply-asserted GWT scenarios. The two deferred error types (MissingJournal, TailMismatch) are honestly documented as State 11 additions. The plan correctly handles the test-first bead scope and provides viable infrastructure for the test writer.

STATUS: APPROVED
