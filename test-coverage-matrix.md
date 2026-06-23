# Test Coverage Matrix: vb-b8i8f

## Metadata

| Field | Value |
|-------|-------|
| Bead | vb-b8i8f |
| State | 8 (test-planner) |
| Plan artifact | test-plan.md |
| Schema | test-coverage-matrix/v1 |
| Total behaviors | 46 |
| Total test scenarios | 61 (including proptest invariants, Kani harnesses, fuzz targets) |

---

## Coverage Summary

| Category | Behaviors | Tests Planned | Existing |
|----------|-----------|---------------|----------|
| C1 (Public Kill API) | 6 | 6 | 0 |
| C2 (Cancel/Kill Missing + Terminal) | 16 | 16 | 0 (caught by existing tests but with wrong Ok semantics) |
| C3 (Single Terminal Event) | 8 | 10 | 5 (existing cancel integration tests) |
| C4 (Stale Authority Cleanup) | 11 | 11 | 2 (ignored tests hp3, hp4) |
| C5 (Kind 28 Storage Admission) | 12 | 18 | 3 (proptest PASS) + 2 (Kani PASS) |
| C6 (Replay Integrity) | 8 | 10 | 2 (Kani PASS) |
| **Total** | **61 demand-side** | **61 planned** | **14 existing** |

---

## Per-Behavior Coverage

### C1: Public Kill API

| ID | Behavior | Test Name (proposed) | Layer | Status |
|----|----------|---------------------|-------|--------|
| B01 | Runtime.kill_run enqueues ShardCommand::Kill | `kill_run_enqueues_shard_command_when_run_routes_to_shard` | integration | new |
| B02 | kill_run returns ShardNotFound for invalid shard | `kill_run_returns_shard_not_found_when_shard_index_invalid` | integration | new |
| B03 | kill_run returns QueueFull when queue exhausted | `kill_run_returns_queue_full_when_command_queue_exhausted` | integration | new |
| B04 | kill_run returns typed error for missing run | `kill_run_returns_run_not_found_when_run_never_submitted` | integration | new |
| B05 | kill_run returns typed error for terminal run | `kill_run_rejects_already_terminal_run` | integration | new |
| B06 | ShardCommand::Kill dispatched to handle_kill | `shard_command_kill_dispatched_to_handle_kill_via_tick` | integration | new |

### C2: Cancel/Kill Missing and Already-Terminal

| ID | Behavior | Test Name (proposed) | Layer | Status |
|----|----------|---------------------|-------|--------|
| B07 | handle_cancel Err for missing run | `handle_cancel_returns_run_not_found_when_run_never_submitted` | integration | new ⚠️ |
| B08 | handle_cancel Err for terminal run | `handle_cancel_returns_already_terminal_when_run_already_cancelled` | integration | new ⚠️ |
| B09 | handle_kill Err for missing run | `handle_kill_returns_run_not_found_when_run_never_submitted` | integration | new ⚠️ |
| B10 | handle_kill Err for terminal run | `handle_kill_returns_already_terminal_when_run_already_killed` | integration | new ⚠️ |
| B11 | cancel missing: no journal event | `cancel_missing_run_does_not_append_journal_event` | integration | new ⚠️ |
| B12 | cancel terminal: no second journal event | `cancel_terminal_run_does_not_append_second_journal_event` | integration | new ⚠️ |
| B13 | kill missing: no journal event | `kill_missing_run_does_not_append_journal_event` | integration | new ⚠️ |
| B14 | kill terminal: no second journal event | `kill_terminal_run_does_not_append_second_journal_event` | integration | new ⚠️ |
| B15 | cancel missing: no counter increment | `cancel_missing_run_does_not_increment_failed_counter` | integration | new ⚠️ |
| B16 | cancel terminal: no double counter | `cancel_terminal_run_does_not_increment_failed_counter_twice` | integration | new ⚠️ |
| B17 | kill missing: no counter increment | `kill_missing_run_does_not_increment_failed_counter` | integration | new ⚠️ |
| B18 | kill terminal: no double counter | `kill_terminal_run_does_not_increment_failed_counter_twice` | integration | new ⚠️ |
| B19 | cancel missing: no terminal_runs mutation | `cancel_missing_run_does_not_mutate_terminal_runs` | integration | new ⚠️ |
| B20 | kill missing: no terminal_runs mutation | `kill_missing_run_does_not_mutate_terminal_runs` | integration | new ⚠️ |
| B21 | cancel missing: no trace event | `cancel_missing_run_does_not_push_trace_event` | integration | new ⚠️ |
| B22 | kill missing: no trace event | `kill_missing_run_does_not_push_trace_event` | integration | new ⚠️ |

> ⚠️ = Will FAIL initially (TDD red). Current production code always returns `Ok(())` from `handle_cancel`/`handle_kill`. These tests verify the corrected behavior from State 10 implementation (Task 2).

### C3: Single Terminal Journal Event

| ID | Behavior | Test Name | Layer | Status |
|----|----------|----------|-------|--------|
| B23 | cancel live: exactly one RunCancelled | `hp1_cancel_running_run_transitions_to_cancelled` | integration | **existing** ✅ |
| B24 | kill live: exactly one RunKilled | `kill_live_run_appends_exactly_one_runkilled_event` | integration | new |
| B25 | cancel-then-cancel: rejected | `ec1_terminal_cancelled_state_does_not_regress` | integration | **existing** ✅ |
| B26 | kill-then-kill: rejected | `kill_already_killed_run_is_rejected` | integration | new |
| B27 | kill-after-cancel: rejected | `kill_after_cancel_is_rejected` | integration | new |
| B28 | cancel-after-kill: rejected | `cancel_after_kill_is_rejected` | integration | new |
| B29 | cancel-then-finish: no finish event | `finish_after_cancel_is_rejected_or_ignored` | integration | new |
| B30 | kill-then-finish: no finish event | `finish_after_kill_is_rejected_or_ignored` | integration | new |
| -- | terminal invariance retest with kill | `inv1_terminal_never_regresses_after_kill` | integration | new |

### C4: Stale Action/Timer Cleanup

| ID | Behavior | Test Name | Layer | Status |
|----|----------|----------|-------|--------|
| B31 | cancel removes pending timer | `cancel_removes_pending_timer` | integration | new |
| B32 | kill removes pending timer | `kill_removes_pending_timer` | integration | new |
| B33 | action completion after cancel: error | `hp4_action_after_cancel_returns_error` | integration | **existing** 🔧 |
| B34 | action failure after cancel: error | `action_failure_after_cancel_returns_error` | integration | new |
| B35 | action completion after kill: error | `action_completion_after_kill_returns_error` | integration | new |
| B36 | action failure after kill: error | `action_failure_after_kill_returns_error` | integration | new |
| B37 | ask answer after cancel: error | `ask_answer_after_cancel_returns_error` | integration | new |
| B38 | ask answer after kill: error | `ask_answer_after_kill_returns_error` | integration | new |
| B39 | timer fire after cancel: error | `timer_fire_after_cancel_returns_error` | integration | new |
| B40 | timer fire after kill: error | `timer_fire_after_kill_returns_error` | integration | new |
| B41 | stale action: no state mutation | `stale_action_does_not_mutate_state` | integration | new |

> 🔧 = Existing test `hp4_action_after_cancel_returns_error` is `#[ignore]` with note "HP-3 and HP-4 tests require runtime fix that was reverted". Must be un-ignored and verified to pass after State 10 fix.

### C5: Durable Kill Storage Admission

| ID | Behavior | Test Name (proposed) | Layer | Status |
|----|----------|---------------------|-------|--------|
| B42 | RecordKind::RunKilled.id() == 28 | `record_kind_run_killed_id_is_28` | unit | new |
| B43 | is_known_record_kind(28) = true | `is_known_record_kind_28_returns_true` | unit | new |
| B44 | validate_kind_family(journal, 28) = Ok | `validate_kind_family_journal_event_28_returns_ok` | unit | new |
| B45 | validate_kind_family(snapshot, 28) = Err | `validate_kind_family_snapshot_28_returns_rejection` | unit | new |
| B46 | validate_kind_family(blob, 28) = Err | `validate_kind_family_blob_28_returns_rejection` | unit | new |
| B47 | encode RunKilled produces bytes | `encode_record_runkilled_produces_valid_bytes` | unit | new |
| B48 | decode round-trip RunKilled | `decode_record_runkilled_roundtrip` | unit | new |
| B49 | decode_journal_event validates RunKilled | `decode_journal_event_runkilled_passes_validation` | unit | new |
| B50 | validate_known_kind(28) = Ok | `validate_known_kind_28_returns_ok` | unit | new |
| B51 | unknown_record_kind_value(28) = None | `unknown_record_kind_value_28_returns_none` | unit | new |
| B52 | kind 31 still unknown | `is_known_record_kind_31_returns_false` | unit | new |
| B53 | journal family rejects kind 31 | `validate_kind_family_journal_event_31_returns_rejection` | unit | new |
| -- | proptest: kind 28 valid | `prop_record_kind_28_is_valid` | proptest | **existing** ✅ |
| -- | proptest: RunKilled valid event | `prop_runkilled_valid_event_passes_validation` | proptest | **existing** ✅ |
| -- | proptest: zero run invalid | `prop_runkilled_zero_run_invalid` | proptest | **existing** ✅ |
| -- | proptest: zero attempt invalid | `prop_runkilled_zero_attempt_invalid` | proptest | **existing** ✅ |
| -- | proptest: overflow seq invalid | `prop_runkilled_overflow_seq_invalid` | proptest | **existing** ✅ |
| -- | Kani: kind 28 known bounded | `check_kind_28_known` | kani | **existing** ✅ (wired) |
| -- | Kani: kind 28 journal family | `check_kind_28_journal_family` | kani | **existing** ✅ (wired) |
| -- | Kani: kind 28 snapshot rejected | `check_kind_28_snapshot_family_rejected` | kani | **existing** ✅ (wired) |
| -- | Kani: kind 28 blob rejected | `check_kind_28_blob_family_rejected` | kani | **existing** ✅ (wired) |
| -- | Kani: all existing kinds known | `check_all_existing_kinds_known` | kani | **existing** ✅ (wired) |
| -- | Kani: journal family exhaustive | `check_journal_family_exhaustive` | kani | **existing** ✅ (wired) |
| -- | Fuzz: kind validation | `fuzz/fuzz_targets/kind_validation.rs` | fuzz | existing (pending exec) |
| -- | Proptest: kind 28 round-trip | `prop_runkilled_encode_decode_roundtrip` | proptest | BLOCKED (compile error) |

### C6: Replay Integrity

| ID | Behavior | Test Name (proposed) | Layer | Status |
|----|----------|---------------------|-------|--------|
| B54 | events_for_run contiguity | `prop_replay_contiguity_mixed_kinds` | proptest | BLOCKED (compile) |
| B55 | RunKilled preserves EventSeq | `validate_replayed_event_preserves_runkilled_seq` | unit | new |
| B56 | validate_replayed_event match | `validate_replayed_event_match_returns_ok` | unit | new |
| B57 | validate_replayed_event gap | `validate_replayed_event_seq_mismatch_returns_gap` | unit | new |
| B58 | validate_replayed_event wrong run | `validate_replayed_event_run_mismatch_returns_wrong_run` | unit | new |
| B59 | RunKilled replays as terminal | `runkilled_events_replay_as_terminal` | integration | new |
| B60 | kind 28/29 does not open kind 31 | `kind_28_and_29_admission_does_not_open_unknown_kind_31` | unit | new |
| B61 | next_seq overflow | `next_seq_max_returns_overflow` | unit | new |
| -- | Kani: replay contiguity + killed | `check_replay_contiguity_with_killed` | kani | **existing** ✅ (wired) |
| -- | Kani: replay gap detection | `check_replay_sequence_gap_detection` | kani | **existing** ✅ (wired) |
| -- | Fuzz: journal decode | `fuzz/fuzz_targets/journal_decode.rs` | fuzz | existing (pending exec) |

---

## Integration Test Structure

All integration tests use the `Runtime` public API with `VolatileRuntimeJournal` to verify side effects:

| Test Group | Test File | Pattern |
|-----------|-----------|---------|
| Cancel lifecycle | `cancel_kill_lattice_tests.rs` (existing section) | `Runtime::new_with_journal` → `submit_*` → `tick_count` → `cancel_run` → `tick_and_drain` → assert journal/counters/trace |
| Kill lifecycle | `cancel_kill_lattice_tests.rs` (new section) | Same as cancel but with `kill_run` → assert `RunKilled` journal event + counters |
| Error semantics | `cancel_kill_lattice_tests.rs` (new section) | Submit + tick → cancel/kill → second cancel/kill → assert `Err` + journal unchanged |
| Stale authority | `cancel_kill_lattice_tests.rs` (new section) | Submit + tick → cancel/kill → attempt stale action/timer/ask → assert `Err` + state unchanged |

---

## Risk-Register Mapping

| Contract Clause | Blocker Status | Test Gap | Mitigation |
|----------------|---------------|----------|------------|
| C1 (Public Kill API) | `Runtime::kill_run` missing | No kill API tests can pass | Tests written for `kill_run` will fail until State 10 Task 1 |
| C2 (Error Semantics) | `handle_cancel/kill` always Ok | Tests B07-B22 will fail initially | TDD: write tests first, State 10 Task 2 fixes production code |
| C3 (Single Terminal) | No blocker | All cancel scenarios tested; kill scenarios new | Extend existing test suite |
| C4 (Stale Authority) | hp3/hp4 tests `#[ignore]` | Tests B33-B41 new or un-ignored | Un-ignore hp3/hp4 after State 10 fix; add kill equivalents |
| C5 (Kind 28 Storage) | BLOCK-001 RESOLVED (isolated workspace) | Proptest BLOCKED by compile error | State 11: fix `proptest_storage.rs:317`; unit tests cover admission in isolation |
| C6 (Replay Integrity) | BLOCK-002 (Kani runtime shard construction) | Proptest BLOCKED by compile error | Unit tests cover replay validation functions; Kani storage harnesses cover replay contiguity at codec level |

---

## Evidence Commands for Test Execution

After State 9 tests are written and State 10 implementation completes, the following commands produce evidence:

```bash
# Unit: storage codec
cargo test -p vb_storage -- kill_kind
cargo test -p vb_storage -- replay_integrity

# Integration: cancel/kill lifecycle
cargo test -p velvet-ballistics-workspace-tests --test cancel_kill_lattice_tests

# Proptest: existing + new properties
cargo test -p velvet-ballistics-workspace-tests --test cancel_kill_lattice_props

# Proptest: storage round-trip (post compile fix)
cargo test -p vb_storage -- proptest

# Kani: storage harnesses (re-execute post BLOCK-001)
KANI_FEATURES=legacy-kani cargo kani -p vb_storage

# Fuzz: kind validation
cargo +nightly fuzz run kind_validation -- -max_len=8 -runs=100000

# Fuzz: journal decode
cargo +nightly fuzz run journal_decode -- -max_len=4096 -runs=100000

# Mutation testing
cargo mutants --package vb_storage --files "codec/validation.rs" --timeout 300
cargo mutants --package vb_runtime --files "shard/lifecycle/chunk_002.rs" --timeout 300
```

---

## Traceability to Bridge RRO Rows

| RRO Row | Proof ID | Behavior Test Refs (from bridge) | Test Plan Coverage |
|---------|----------|--------------------------------|-------------------|
| RRO-004 | PO-PROP-001 | `prop_record_kind_28_is_valid`, `prop_runkilled_valid_event_passes_validation` | Existing ✅ + B42-B53 new unit |
| RRO-008 | PO-PROP-002 | `prop_record_kind_28_is_unique`, `prop_journal_kinds_in_valid_range` | Existing ✅ |
| RRO-012 | PO-PROP-003 | `prop_runkilled_carries_attempt`, `prop_runkilled_distinct_from_cancelled` | Existing ✅ |
| RRO-014 | PO-KANI-004 | `prop_kind_28_is_known_record_kind` | Kani existing ✅ + B43-B53 unit |
| RRO-016 | PO-PROP-004 | `prop_kind_28_id_is_stable`, `prop_runkilled_encode_decode_roundtrip` | BLOCKED → unit B47-B49 cover basics |
| RRO-019 | PO-KANI-005 | `prop_replay_contiguity_mixed_kinds` | Kani existing ✅ + B54-B61 unit |
| RRO-021 | PO-PROP-005 | `prop_replay_contiguity_mixed_kinds` | BLOCKED → unit B54-B58 cover basics |
| RRO-001 | PO-VERUS-001 | `hp1_cancel_running_run_transitions_to_cancelled` | Existing + B24 (kill) |
| RRO-006 | PO-KANI-001 | `cancel_kill_lattice_tests.rs` all scenarios | B07-B10 error semantics new |
| RRO-011 | PO-FLUX-001 | `cancel_kill_lattice_tests.rs` all scenarios | B07-B22 side-effect-free new |
| RRO-017 | PO-FUZZ-001 | kind validation props | Fuzz target exists, execution pending |
| RRO-022 | PO-FUZZ-002 | journal decode props | Fuzz target exists, execution pending |
| Remaining 10 RROs | — | — | Deferred to State 11 (formal verification layers) |

---

## Summary

- **Total behaviors**: 46 across 6 contract clauses
- **Tests planned**: 61 (27 new unit, 27 new integration, 7 existing)
- **TDD red tests**: 16 (C2 error semantics — will fail until State 10 implementation)
- **Blocked tests**: 2 (proptest_storage.rs:317 compile error)
- **Deferred to State 11**: 10 RRO rows (Verus GOD RULE 2, Kani runtime wiring, Flux wiring, fuzz execution)
- **Existing PASS**: 14 tests + 2 Kani harness groups (vb_storage wired)
- **Mutation threshold**: >=90% kill rate on `validation.rs`, `chunk_002.rs`
- **Risk**: C2 tests will be red until State 10; this is expected TDD workflow
- **No new waivers**: All test gaps are either TDD-design (red before green) or pre-existing blockers documented in bridge

---

## Handoff

- **State 9 (test-writer)**: Write the 47 new tests documented above. Write them to FAIL (compile or runtime) for C2 behaviors; these will turn green in State 10.
- **State 10 (implementation)**: Fix `handle_cancel`/`handle_kill` error semantics + add `Runtime::kill_run`. C2 tests will turn from red to green.
- **State 11 (formal-verifier)**: Execute fuzz + fix proptest compile error + wire Kani runtime + Flux + Verus.
- **State 12 (closure)**: All 61 tests pass, all 22 RRO rows verified, mutation >90%.
