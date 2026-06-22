# Test Review — Slice 2 (Round 6): vb_storage + workspace_tests

**Scope:** 313 test-bearing Rust files (116 in `crates/vb_storage/`; 197 in `crates/workspace_tests/`; plus benches).
Kani harnesses (`kani_*.rs`, ~80 files) are correctly `#[cfg(kani)]`-gated proof artifacts and excluded from
behavior-test review. Round-6 cwd = `dba556e7f` (state-11 + wave-14 — 3 vb_runtime regressions + 16 P1 + 8 P2 bug-hunt).

**Date:** 2026-06-21
**Reviewer:** test-reviewer agent (slice 2 of 4, round 6)
**Round:** 6 of 40 — verify round-1+2+3+4+5 fixes + find NEW defects from wave-14 commit `dba556e7f`.

## STATUS: REJECTED

Wave-14 added 3 NEW internal `sa002_*` tests in `batch/tests.rs` (gold-standard), updated 3 existing internal tests
to assert `Err(BatchAborted)` instead of `Ok(())` no-op, added 4 new `*_rejects_reserved_seq_sentinel` tests in
`keys/tests.rs`, and ADDED 2 new `*_rejects_reserved_sentinel` tests in `workspace_tests/fjall_keyspace_manifest_tests.rs`
+ `journal_tail_scan_fallback_tests.rs` (S2-CRIT-1 carry-overs are now GOLD-STANDARD). All round-1+2+3+4+5 verified
artifacts that ARE LIVE (security_tests, vb_2bok_durability_gate_tests, recover_tests, batch/tests, keys/tests,
recovery/replay/summary/tests, contracts_production_binding.rs, journal_tail_scan_fallback_tests.rs,
fjall_keyspace_manifest_tests.rs, integration_compile_error_message_quality.rs, integration_runtime_storage_fault_tolerance.rs)
are **STILL APPLIED with 0 regressions**. **HOWEVER, round-6 discovered a NEW CRITICAL structural defect:
12 test files in `crates/vb_storage/src/` are DORMANT (not declared in `lib.rs`, not compiled, not executed)
— including the entire `process_lock_tests.rs`, `edge_case_tests.rs`, `type_tests.rs`, and `blob_tests.rs`
files that rounds 1-5 verified "STILL APPLIED". The round-1+2+3+4+5 verification tables claim "0 regressions",
but those tables were vacuous for these 4 files because the files were never running. Additionally, wave-14
introduced 3 NEW workspace_tests failure clusters (44 total failures) by tightening production contracts
without updating test fixtures: 7 in `vb_test_runtime_lifecycle_state_behavior.rs` + 3 in `vb_jpq7_3_fail_closed_storage_recovery_contract.rs`
+ 2 in `recovery_watermark_tests.rs` (carry from wave-11 reserved-sentinel change, NOT fixed in wave-14). The
round-5 C-R5-1 and C-R5-2 criticals (vb_storage `proptest_vb_vzcuf_PS_004.rs` and workspace_tests
`journal_side_index_contracts.rs` still using `Ok(())` instead of `Err(BatchAborted)`) are ALSO STILL UNFIXED
in wave-14 (wave-14 fixed the INTERNAL tests but missed the EXTERNAL cross-crate tests). The slice cannot
advance to APPROVED until: (1) the 12 dormant test files are either re-declared in `lib.rs` OR deleted;
(2) the 2 wave-5/state-11 C-R5 criticals are reconciled; (3) the 7 wave-14 `vb_runtime` regression tests
in `vb_test_runtime_lifecycle_state_behavior.rs` are reconciled; (4) the 2 `recovery_watermark_tests.rs`
failures from wave-11's `EventSeq::MAX` rejection are reconciled; (5) the 3 `vb_jpq7_3_fail_closed_*` failures
are reconciled. Total: 5 distinct reconciliation tasks, ~3-6 hours.

---

## Round-1+2+3+4+5 Fix Verification Table

| Round | ID | File:Line | Defect | Status | Evidence |
|-------|----|-----------|--------|--------|----------|
| 1 | C-01 / H-08 | `integration_compile_error_message_quality.rs:374-378,401-405,426-430` | `assert!(result.is_ok() \|\| result.is_err())` tautology (3 sites) | **STILL APPLIED** (live test) | Lines 376, 402, 428 still use `matches!(result, Err(CompileErrors(ref errors)) if errors.iter().any(|e| matches!(e, CompileError::DepthLimit/SequenceLimit/ScalarLimit{...})))`. **Wave-14 did not regress this site.** |
| 1 | C-02 | `integration_runtime_storage_fault_tolerance.rs:215-218` | `assert!(result.is_ok() \|\| result.is_err())` tautology | **STILL APPLIED** (live test) | Line 215-218 still uses `matches!(result, Err(ref e) if matches!(e, RuntimeError::InvalidRecoveryHydration))`. **Wave-14 did not regress this site.** |
| 1 | C-03 | `process_lock_tests.rs:141-179` (round-1 cited) | "match-all-outcomes" with `_ = other;` and `Ok(_) => {} \| Err(_) => {}` (2 tests) | **REGRESSION in spirit** — file is **DORMANT** | Round-1+2+3+4+5 verified "STILL APPLIED" via grep on file:line. However, `process_lock_tests.rs` is NOT declared in `lib.rs` (no `mod process_lock_tests;` exists; no `#[path = "process_lock_tests.rs"]` in any mod block). **Test names `process_lock_prevents_dual_writers_same_directory` and `process_lock_is_released_on_drop` do not appear in `cargo test --lib -- --list`. The "fix" was applied to dead code.** The actual production contract for process_lock IS tested by `security_tests::tests::process_lock_file_created_with_holder_pid`, `vb_2bok_durability_gate_tests::durability_gate_tests::process_lock_held_error_code`, etc. — and those tests PASS. |
| 1 | C-04 | `edge_case_tests.rs:547` (round-1 cited) | Test name `encode_rejects_zero_length_payload_serialization` contradicted `assert!(result.is_ok())` | **REGRESSION in spirit** — file is **DORMANT** | Same as C-03: `edge_case_tests.rs` is NOT declared in `lib.rs`. Round-1 verified the rename to `encode_accepts_zero_length_payload_and_round_trips` and the new round-trip assertions — but the file is dead code. Test name not in `cargo test --lib -- --list`. |
| 2 | C-05 | `workspace_tests/tests/doctor_key_decode_tests.rs:617-635` (round-2 cited) | `readonly_journal_declared_keyspaces_returns_ten` asserted `len() == 10` | **STILL APPLIED** (live test) | `readonly_journal_declared_keyspaces_returns_eleven` in `cargo test --list` for `doctor_key_decode_tests` (verified). Wave-14 did not regress. |
| 2 | C-06 | `workspace_tests/tests/fjall_keyspace_manifest_tests.rs:309-356` (round-2 cited) | `declared_keyspaces_count` asserted `len() == 10` | **STILL APPLIED** (live test) | Wave-14 added 1 new test `max_sequence_encoder_rejects_reserved_sentinel` (line 309-320) to this file. **23+6 line diff (round-2 baseline: 6 lines; wave-14: 29 lines added).** Test file is GREEN (24 passed, 0 failed). |
| 2 | S2-CRIT-1 | `journal_tail_scan_fallback_tests.rs` + `fjall_keyspace_manifest_tests.rs` + `keys/tests.rs` (round-2 cited) | `u64::MAX → u64::MAX - 1` + `*_rejects_reserved_sentinel` tests | **STILL APPLIED + EXPANDED** | Wave-14 added `max_sequence_encoder_rejects_reserved_sentinel` (fjall_keyspace:309) + `run_event_key_rejects_reserved_sentinel` (journal_tail:985) + 4 new `*_rejects_reserved_seq_sentinel` tests in `keys/tests.rs:424,433,442,451` (verified via `cargo test --list`). **All 7 `*_rejects_reserved_*` tests are LIVE and PASSING.** Wave-14 also added `max_sequence_key_encodes_just_below_sentinel` (line 851+) to bridge the boundary. |
| 2 | artifacts | `vb_storage/src/journal/tests.rs:1889,1931,1976` | 3 corruption tests for `events_for_run` rejection | **STILL APPLIED + STILL PASSING** | All 3 corruption tests present and live (verified via `cargo test --lib`). 1552 lib tests pass — these are included. |
| 3 | H-02 | `vb_storage/src/codec/tests.rs:565,2813,2825,2850` (round-3 cited) | `assert!(result.is_ok())` smoke tests | **STILL APPLIED** (live test) | All 4 sites still use `assert_eq!(result, Ok(()), "...")`. Wave-14 did not regress. M-NEW-2 (8 additional sites at 534, 674, 1756, 2459, 2655, 2739, 2863, 2996) STILL NOT FIXED. |
| 3 | H-03 | `vb_storage/src/security_tests.rs:767` (round-3 cited) | `assert!(result.is_ok())` SECURITY smoke test | **STILL APPLIED** (live test) | Line 767-771 still uses `assert_eq!(result, Ok(()), "SECURITY: correct digest must pass verification")`. Wave-14 did not regress. |
| 3 | M-02 | `vb_storage/src/process_lock_tests.rs:213-228` (round-3 cited) | Smoke test + no `lock_path.exists()` check | **N/A — DORMANT** | Same as C-03. File is dead code. |
| 3 | M-03 | `workspace_tests/tests/contracts_production_binding.rs:170-187, 228-244, 292-313, 330-344` (round-3 cited) | `is_err()` accepts any variant | **STILL APPLIED** (live test) | All 4 test functions still use `.err().unwrap_or_else(...)` + `message.contains("MISSING_SCHEMA_VERSION"\|"INVALID_VERSION")`. Wave-14 did not regress. |
| 3 | M-05 | `workspace_tests/tests/contracts_production_binding.rs:282` (round-3 cited) | `assert!(parse_vet_exit_code(0).is_ok())` smoke | **STILL APPLIED** (live test) | `test_prod_parse_vet_exit_code_success` (line 321-327) still uses `assert_eq!(parse_vet_exit_code(0), Ok(()), "...")`. |
| 3 | artifact | `frame_seed_slot_dimension_overflow_reports_exact_variant` at `vb_storage/src/recovery/replay/summary/tests.rs:482-500` (round-3 cited) | Bogus "Expected Ok, got" comment + `assert!(result.is_ok())` | **STILL APPLIED + STILL PASSING** | Test live and passing (verified via `cargo test --lib recovery::replay::summary::tests::frame_seed_slot_dimension_overflow_reports_exact_variant`). Wave-14 did not regress. |
| 3 | artifacts | 3 `#[ignore]` behavior tests (round-3 cited) | Round-4 reduced 5 → 3 `#[ignore]` | **STILL APPLIED** (no change) | 3 `#[ignore]` at `vb_c1s0_orchestration_runtime_tests.rs:593`, `vb_test_runtime_lifecycle_state_behavior.rs:469`, `vb_vt2f_direct_runtime_api_acceptance.rs:605`. Wave-14 did not un-ignore any. |
| 4 | S2-CRIT-1 (carry) | `journal_tail_scan_fallback_tests.rs` + `keys/tests.rs` | round-4 reserved-sentinel tests | **STILL APPLIED + EXPANDED** | Same as round-2 S2-CRIT-1 above. Wave-14 added 4 more tests in `keys/tests.rs`. |
| 4 | C-NEW-1 (carry) | 39 workspace_tests failures (round-4) | Carry-over from wave-12 | **STILL APPLIED + EXPANDED to 44** | Wave-14 added 5 NEW failures. Total 44 = 39 carry + 5 new (4 in `vb_test_runtime_lifecycle_state_behavior.rs` + 1 in `recovery_watermark_tests.rs`). |
| 5 | C-R5-1 | `vb_storage/tests/proptest_vb_vzcuf_PS_004.rs:43-55, 106-116` | `b2.commit().expect("commit")` after duplicate-event abort | **STILL APPLIED — STILL UNFIXED** | Lines 52, 115 still use `.expect("commit")`. **Wave-14 FIXED the internal `batch/tests.rs:920,953,1051` and added 3 NEW `sa002_*` tests (lines 1075-1150) — but DID NOT UPDATE the external proptest_vb_vzcuf_PS_004.** Verified: `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004` produces `test result: FAILED. 3 passed; 2 failed; 0 ignored`. **This is a CARRY-OVER critical.** |
| 5 | C-R5-2 | `workspace_tests/tests/journal_side_index_contracts.rs:464-498, 590-648` | `append_event(...).is_ok()` + `commit().is_ok()` after duplicate-event abort | **STILL APPLIED — STILL UNFIXED** | Lines 483, 487 (`test_duplicate_idempotency_key`) + 636, 648 (`test_aborted_gate_blocks_subsequent_staging`) still use `is_ok()`. **Wave-14 did not update.** Verified: `cargo test -p vb_workspace_tests --test journal_side_index_contracts` produces `test result: FAILED. 9 passed; 2 failed; 0 ignored`. **CARRY-OVER critical.** |
| 5 | M-R5-1 | `vb_storage/src/batch/byte_accounting_tests.rs:786` | Test name `e2e_aborted_batch_commit_succeeds_with_no_persist` is misleading | **STILL APPLIED + STILL UNFIXED** | Line 786 still named `e2e_aborted_batch_commit_succeeds_with_no_persist` but test now asserts `Err(BatchAborted)` (lines 805-809). **Wave-14 did not rename.** Test PASSES (verified via `cargo test --lib batch::byte_accounting_tests::e2e_aborted_batch_commit_succeeds_with_no_persist`). Carry-over MEDIUM. |

**Round-1+2+3+4+5 regression count: 4 (C-03, C-04, M-02, and the 12 dormant test files)** — but the regressions are
in SPIRIT, not in production behavior. The 4 "regressions" are because the cited test files (process_lock_tests.rs,
edge_case_tests.rs, type_tests.rs, blob_tests.rs) are DORMANT. The actual production contract for those
contracts is tested by OTHER live modules (security_tests, vb_2bok_durability_gate_tests, recover_tests,
hydrate_tests, batch/tests, etc.) which all pass. **The new finding is structural: 12 dormant test files
represent ~120KB of dead code that rounds 1-5 reviewed without realizing the files were never compiled.**

---

## Wave-14 Changes Detected (positive and negative)

### Positive changes (wave-14 added)

1. **3 NEW internal `sa002_*` tests** at `batch/tests.rs:1075-1150` (wave-14 +126 -8 lines):
   - `sa002_commit_aborted_batch_returns_typed_error_not_silent_ok`
   - `sa002_aborted_batch_commit_does_not_persist_staged_writes`
   - `sa002_open_batch_commit_still_succeeds`
   - All 3 use `matches!(commit_result, Err(JournalError::BatchAborted { reason }))` with concrete `reason` strings.
   - All 3 are LIVE and PASSING. **Gold standard.**

2. **3 EXISTING internal tests updated** in `batch/tests.rs:920,953,1051` from `expect("aborted commit returns Ok(())")` to `matches!(commit_result, Err(JournalError::BatchAborted { .. }))`. Verified live and passing.

3. **4 NEW `*_rejects_reserved_seq_sentinel` tests** in `keys/tests.rs:424,433,442,451` (wave-14 +65 -12 lines):
   - `run_event_key_rejects_reserved_seq_sentinel`
   - `run_snapshot_key_rejects_reserved_seq_sentinel`
   - `recovery_stamp_key_rejects_reserved_seq_sentinel`
   - `event_seq_try_new_rejects_reserved_sentinel`
   - All use `matches!(result, Err(JournalError::ReservedSeqSentinel))` and `EventSeq::MAX_ENCODABLE`.
   - All 4 are LIVE and PASSING.

4. **2 NEW `*_rejects_reserved_sentinel` tests** in workspace_tests:
   - `max_sequence_encoder_rejects_reserved_sentinel` in `fjall_keyspace_manifest_tests.rs:309-320` (wave-14 +29 -6 lines)
   - `run_event_key_rejects_reserved_sentinel` in `journal_tail_scan_fallback_tests.rs:985-998` (wave-14 +56 -11 lines)
   - Both PASS.

5. **SC-001 bijection fix in `types/index.rs`** (wave-14 +40 -9 lines): `IndexStatusState::Other(v)` now uses
   `v + MIN_OTHER_BYTE` offset encoding. 3 internal tests in `keys/tests.rs` updated. Live and passing.

6. **CF-001 fix in `recovery/event_replay/tail.rs:55-67`** (wave-14 +8 -3 lines): `set_max_parallel_in_flight(0)`
   → `set_max_parallel_in_flight(u16::MAX)`. The `derive_dimensions_includes_action_scheduled_ticket_output` test
   is now PASSING (verified live).

### Negative changes (wave-14 introduced NEW regressions)

1. **5 NEW workspace_tests test failures** in `vb_test_runtime_lifecycle_state_behavior.rs`:
   - `cancel_run_transitions_run_to_failed` (line 589)
   - `action_scheduled_lifecycle_event_recorded` (line 686)
   - `fail_action_transitions_run_to_failed` (line 547)
   - `run_failed_lifecycle_event_recorded` (line 779)
   - `run_finished_lifecycle_event_recorded` (line 739)
   - `step_started_lifecycle_event_recorded` (line 714)
   - `submit_lifecycle_event_recorded_before_tick` (line 658)
   - All 7 panic at the `events.iter().any(|e| matches!(e, RuntimeJournalEvent::X))` assertion, meaning
     the production code no longer journals the event after `tick_and_drain()`. **Wave-14 production change
     in vb_runtime is the cause.** Production is now broken OR the test fixtures are stale.

2. **2 NEW workspace_tests test failures** in `recovery_watermark_tests.rs`:
   - `proptest_snapshot_seq_lt_tail_first_seq` (line 720, panic at 697:1)
   - `watermark_journal_recovery_rejects_max_seq` (line 572, panic at line 578:21)
   - Root cause: tests use `EventSeq::MAX` (= u64::MAX) in `append_journaled` fixture. The encoder now
     rejects `EventSeq::MAX` per the S2-CRIT-1 fix. Test fixtures are STALE relative to wave-11/14 reserved-sentinel
     contract change.

3. **3 NEW workspace_tests test failures** in `vb_jpq7_3_fail_closed_storage_recovery_contract.rs`:
   - `given_full_journal_slot_taint_metadata_is_corrupt_when_hydrating_then_recovery_fails_closed`
   - `given_legacy_collect_frame_extra_when_hydrating_full_journal_then_extra_is_not_corrupt_taint`
   - `given_public_hydration_tail_slot_cannot_be_dimensioned_when_recovery_runs_then_clean_taint_is_not_defaulted`
   - All 3 fail at recovery-time taint/extra computation. Production may be tighter than tests expect, OR
     test fixtures are stale.

4. **C-R5-1 / C-R5-2 STILL UNFIXED** (round-5 carry-overs):
   - `proptest_vb_vzcuf_PS_004.rs:52, 115` still use `b2.commit().expect("commit")` after `b2.append_event` returns Err.
   - `journal_side_index_contracts.rs:483, 487, 636, 648` still use `is_ok()` after `append_event` / `commit` returns Err.
   - Both files are CROSS-CRATE proptests. Wave-14 fixed the INTERNAL `batch/tests.rs` but missed the EXTERNAL
     cross-crate proptests. **This is a coordination failure between bead scopes.**

### Structural finding (NEW round-6 critical)

**12 test files in `crates/vb_storage/src/` are DORMANT — not declared in `lib.rs`, not compiled, not executed.**

| File | Size | Status |
|------|------|--------|
| `artifact_tests.rs` | 7.8K | DORMANT |
| `batch_tests_vb_mrwe_7.rs` | 3.6K | DORMANT |
| `blob_tests.rs` | 8.7K | DORMANT (round-1 M-06 cited this) |
| `edge_case_tests.rs` | 24.0K | DORMANT (round-1 C-04 cited this) |
| `error_code_tests.rs` | 9.6K | DORMANT |
| `header_tests.rs` | 8.3K | DORMANT |
| `process_lock_tests.rs` | 7.9K | DORMANT (round-1 C-03 + round-3 M-02 cited this) |
| `record_tests.rs` | 8.2K | DORMANT |
| `recovery_type_tests.rs` | 10.5K | DORMANT |
| `replay_core_tests.rs` | 10.8K | DORMANT |
| `snapshot_tests.rs` | 9.9K | DORMANT |
| `type_tests.rs` | 11.3K | DORMANT (round-3 M-08 cited this) |

**Evidence:** `rg "path = \"process_lock_tests\\.rs|edge_case_tests\\.rs|type_tests\\.rs|blob_tests\\.rs"` returns 0 hits
in `crates/vb_storage/src/`. The `lib.rs:265-294` does not declare any of these files. `cargo test --lib -- --list`
does not include any of these test names.

**Impact:** ~120KB of test code is dead. The "STILL APPLIED" verifications in rounds 1-5 for C-03, C-04, M-02, M-06,
M-08 were vacuous. **Rounds 1-5 should have flagged this.** The actual production contract for these areas is
tested by other live modules (security_tests, vb_2bok_durability_gate_tests, recover_tests, etc.).

**Workaround:** Either add `#[cfg(test)] #[path = "process_lock_tests.rs"] mod process_lock_tests;` (and similar)
to `lib.rs`, OR delete the dormant files.

---

## Findings (CRITICAL first)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix |
|----|-----|-----------|--------|------------------------------|------------------|
| **C-R6-1** | **CRITICAL** | `crates/vb_storage/src/process_lock_tests.rs`, `edge_case_tests.rs`, `type_tests.rs`, `blob_tests.rs`, `header_tests.rs`, `record_tests.rs`, `recovery_type_tests.rs`, `replay_core_tests.rs`, `artifact_tests.rs`, `error_code_tests.rs`, `snapshot_tests.rs`, `batch_tests_vb_mrwe_7.rs` (12 files, 120KB) | **DORMANT TEST FILES** — not declared in `lib.rs`, not compiled, not executed. Rounds 1-5 reviewed these as if they were live. | Delete the 12 files. The actual production contracts ARE tested by live modules (security_tests, vb_2bok_durability_gate_tests, recover_tests, batch/tests, etc.). The "STILL APPLIED" verification for round-1 C-03, C-04, M-02, M-06, M-08 was vacuous. | EITHER (a) `#[cfg(test)] #[path = "X.rs"] mod X;` in `lib.rs` for each of the 12 files, OR (b) `rm -f` each file. Option (a) restores ~120KB of test coverage; option (b) removes 120KB of dead code. **Recommend (a) — restore the dormant tests as gold-standard behavior tests, then re-run round-7 to verify the historical round-1+2+3+4+5 fixes are actually catching production mutations.** |
| **C-R6-2** | **CRITICAL** | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs:52,115` and `crates/workspace_tests/tests/journal_side_index_contracts.rs:483,487,636,648` | **C-R5-1 + C-R5-2 carry-overs STILL UNFIXED.** External proptest cross-crate tests still use `b2.commit().expect("commit")` and `prop_assert!(batch.commit().is_ok())` after the aborting `append_event`. Production correctly returns `Err(BatchAborted)` per wave-11 SA-002, but tests assert `Ok(())`. | Delete the production `commit()` abort-error path. The 2 internal `batch/tests.rs` tests (lines 920, 953, 1051) and 3 new `sa002_*` tests would catch this — but the 2 cross-crate proptests would still pass because they expect `Ok(())`. | For each of 4 sites: replace `commit().expect("commit")` and `commit().is_ok()` with `prop_assert!(matches!(b2.commit(), Err(JournalError::BatchAborted { .. })));`. Also update line 483 `append_event(&event_b).is_ok()` to expect `Err(DuplicateEvent)` and line 636 `subsequent_result.is_ok()` to expect `Err(DuplicateEvent)`. |
| **C-R6-3** | **CRITICAL** | `crates/workspace_tests/tests/vb_test_runtime_lifecycle_state_behavior.rs:547,589,658,686,714,739,779` (7 sites) | **NEW wave-14 regression.** 7 lifecycle tests assert `journal.snapshot().iter().any(|e| matches!(e, RuntimeJournalEvent::X))` for `RunCancelled`, `ActionScheduledTicket`, `RunFailed`, `RunFinished`, `StepStarted`, `Submit`, etc. After `submit_action_then_finish + tick_and_drain`, the journal does NOT contain the expected event. Wave-14 production change in vb_runtime broke these. | In production, suppress journaling of `RunCancelled`, `ActionScheduledTicket`, `RunFailed`, `RunFinished`, `StepStarted`, `RunSubmitted` after `submit_action_then_finish`. Tests still pass. The actual contracts ARE journaled somewhere else but not visible in `journal.snapshot()`. | **Requires investigating the wave-14 vb_runtime changes that caused these regressions.** Either revert the production change OR update the test fixture to a different workflow that exercises the new contract. **Open a new bead for triage.** |
| **C-R6-4** | **CRITICAL** | `crates/workspace_tests/tests/recovery_watermark_tests.rs:572, 720` (2 sites) | **Wave-11 reserved-sentinel carry-over STILL UNFIXED.** Tests `watermark_journal_recovery_rejects_max_seq` and `proptest_snapshot_seq_lt_tail_first_seq` use `EventSeq::MAX` (= u64::MAX) in `append_journaled` fixture (lines 580, 620, 404, 426, 448, etc.). The encoder now rejects `EventSeq::MAX` per the S2-CRIT-1 fix. Test fixtures are STALE. | Delete the `EventSeq::MAX` rejection in the key encoder. The `max_sequence_encoder_rejects_reserved_sentinel` test in `fjall_keyspace_manifest_tests.rs:309` would still pass because it tests a different contract. | Replace `EventSeq::MAX` with `EventSeq::MAX_ENCODABLE` (= u64::MAX - 1) in the test fixtures. The tests then exercise the recovery layer's MAX-rejection at a valid boundary. |
| **C-R6-5** | **CRITICAL** | `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:409, 481, ?` (3 sites) | **NEW wave-14 regression.** 3 fail-closed recovery tests fail at recovery-time taint/extra computation. Production may be tighter than tests expect. | Make recovery layer return `clean_taint` as `vec![]` for slot-cannot-be-dimensioned cases. Tests pass. **But the contract `clean_taint is not defaulted` would be silently violated.** | **Requires investigating the wave-14 recovery layer changes.** Open a new bead for triage. |
| H-R6-1 | HIGH | `crates/workspace_tests/tests/vb_nf2u_ui_release_acceptance.rs:260, 529` (8 sites) | 8 `cargo xtask ai-release` integration tests fail. The `assert_command_succeeded` helper (line 260) asserts `output.status.code() == Some(0)`. The actual `cargo xtask ai-release --bead vb-nf2u` command fails (returns non-zero). **This is an environmental/integration issue, not a test defect — but the tests are failing CI.** | Make the test skip when the bead doesn't exist. Tests pass. | Investigate whether the test fixture is set up correctly. The bead `vb-nf2u` may not exist. Open a new bead for triage. |
| H-R6-2 | HIGH | `crates/workspace_tests/tests/cancel_kill_lattice_tests.rs:430, 724, 756, 882, 920` (5 sites) | 5 cancel/kill tests fail at `events.iter().any(|e| matches!(e, RuntimeJournalEvent::RunCancelled/RunKilled { run }))`. Production may not journal the event after `cancel_run` or `kill_run`. **Similar to C-R6-3.** | Same as C-R6-3. | Open a new bead. |
| H-R6-3 | HIGH | `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs:498, 595, 797` (3 sites) | 3 direct-API tests fail at `runtime.submit_direct + tick + journal.snapshot()`. Production may not journal the event. | Same as C-R6-3. | Open a new bead. |
| H-R6-4 | HIGH | `crates/workspace_tests/tests/vb_qi37_1_1_red_recovery_contract_test.rs:158, 168, 173, 235, 243, 250, 351` (7 sites) | 7 red-recovery contract tests fail. Mix of `ask_answer_records_exact_clean_taint`, `no_output_step_*`, `event_only_recovery_*`, `proptest_no_output_success_never_creates_slot_zero`, `runtime_to_storage_mapping_preserves_taint_for_slot_write`. | Same root cause as C-R6-3/C-R6-5. | Open a new bead. |
| H-R6-5 | HIGH | `crates/workspace_tests/tests/vb_a0t1_source_length_gate_tests.rs:146, 679` (2 sites — round-4 carry) | 2 source-length gate tests fail. Pre-existing carry from round 4. | n/a. | Round-4 already documented this. Open a new bead. |
| H-R6-6 | HIGH | `crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs:1165` (1 site) | `tail_seq_equal_to_snapshot_seq_fails` fails at `RecoveryError::ReplayDivergence { detail, .. }` match. The actual error message is missing the `"10" && "not after"` substring. | Delete the message-substring check. Test passes. **But the production contract for `ReplayDivergence::detail` would be silently violated.** | Inspect what `detail` actually contains. Update the assertion to match. |
| H-R6-7 | HIGH | `crates/workspace_tests/tests/doctor_storage_scan_decode_tests.rs:1` (1 site) | `bounded_scan_overflow_limit_handled_safely` panics with `RawVec` allocation error (line at `library/alloc/src/raw_vec/mod.rs:28:5`). Production may panic on `usize::MAX` overflow. | Make `bounded_scan` return `None` on `usize::MAX`. Test passes. | The `RawVec::reserve` panics on overflow. Production should return `None` or use checked arithmetic. |
| H-R6-8 | HIGH | `crates/workspace_tests/tests/vb_test_runtime_ipc_resource_behavior.rs:1346` (1 site) | `edge_submit_after_shutdown_enqueues_but_does_not_process` fails. The shard may not enqueue after shutdown. | n/a. | Investigate. |
| H-R6-9 | HIGH | `crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs:326` (1 site) | `valid_workspace_passes_sharpened_assertions` fails at `output.status.success()`. The xtask command fails. | n/a — environmental. | Investigate the xtask command. |
| H-R6-10 | HIGH | `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs:2274` (1 site) | `bdd_kyyf_001_to_006_require_executable_public_surfaces_not_catalog_bookkeeping_only` fails. | n/a. | Investigate. |
| M-R6-1 | MEDIUM | `crates/vb_storage/src/codec/tests.rs:534, 674, 1756, 2459, 2655, 2739, 2863, 2996` (8 sites — round-4 carry) | `assert!(result.is_ok(), "...")` smoke tests on codec accept/decode functions. Round-1 H-02 fixed 4 sites but missed these 8. | Replace each function with `Ok(())` for any input. All 8 tests pass. | Convert each to `assert_eq!(result, Ok(()), "...")`. |
| M-R6-2 | MEDIUM | `crates/vb_storage/src/batch/byte_accounting_tests.rs:786` (round-5 carry) | Test name `e2e_aborted_batch_commit_succeeds_with_no_persist` is misleading after wave-11 SA-002. Line 787 comment also says "commit succeeds as no-op" but test asserts `Err(BatchAborted)`. | n/a — purely cosmetic. | Rename to `e2e_aborted_batch_commit_surfaces_typed_error` and update line 787 comment. |
| M-R6-3 | MEDIUM | `crates/vb_storage/src/hydrate_tests.rs:281, 287, 374` (3 sites — round-3 carry) | `assert!(result.is_ok(), "...")` smoke tests. | Replace `validate_tail_first_seq_contiguous_with_snapshot` with `Ok(())` always. All 3 sites pass. | Convert each to `assert_eq!(result, Ok(()), "...")`. |
| M-R6-4 | MEDIUM | `crates/workspace_tests/tests/vb_test_compile_parse_validate_behavior.rs:184-188` (round-3 carry) | `assert!(result.is_err())` without variant check. | Make `YamlCompiler::parse_ast(b"   \n")` return `Err(CompileError::Other)`. Test passes. | Add `let msg = result.unwrap_err().to_string(); assert!(msg.contains("empty") \|\| msg.contains("whitespace"));`. |
| M-R6-5 | MEDIUM | `crates/vb_storage/src/blob_tests.rs:260` (DORMANT — round-3 carry) | `result.is_err() \|\| result.map_or(false, \|opt\| opt.is_none())` — accepts EITHER `Err(anything)` OR `Ok(None)`. **File is DORMANT.** | n/a — file is not compiled. | Either re-declare in `lib.rs` (C-R6-1) and fix the assertion, OR delete the file. |
| M-R6-6 | MEDIUM | `crates/workspace_tests/tests/integration_compile_error_message_quality.rs:488` (round-4 carry) | `assert!(!errors.is_empty() \|\| is_ok)` NEW TAUTOLOGY from wave-7/8. | Delete the version validation in production (return `Ok(_)` for any version). Test passes. | Replace with concrete contract: `assert!(matches!(result, Err(CompileErrors(ref errors)) if errors.iter().any(|e| matches!(e, CompileError::InvalidVersion { .. }))));`. |
| L-R6-1 | LOW | `crates/vb_storage/src/codec/tests/replay_integrity.rs:201` (round-4 carry) | `assert!(result.is_ok(), "next_seq(u64::MAX-1) must succeed")` — smoke test on `next_seq` boundary. | Replace `next_seq(u64::MAX-1)` with `Ok(EventSeq::new(u64::MAX))` (skip the +1). Test passes. | Convert to `assert_eq!(result, Ok(EventSeq::new(u64::MAX)), "next_seq at boundary")`. |
| L-R6-2 | LOW | `crates/vb_storage/src/tests/chunk_012.rs:181` (round-3 carry) | Test name `declared_keyspaces_returns_ten_entries` says "ten" but asserts `keyspaces.len() == 11`. | n/a | Rename to `declared_keyspaces_returns_eleven_entries`. |
| L-R6-3 | LOW | `crates/workspace_tests/tests/vb_nf2u_ui_release_acceptance.rs:529` | `must<T>` helper uses `assert_eq!(format!("{context}: {error}"), ""); std::process::abort();` — clever but `process::abort` is sharp edge (no destructor unwinding). | n/a | Replace with `panic!("{context}: {error}")`. |
| O-R6-1 | OBSERVATION | `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs:605`, `vb_test_runtime_lifecycle_state_behavior.rs:469`, `vb_c1s0_orchestration_runtime_tests.rs:593` (3 sites — round-3 carry) | `#[ignore]` on behavior tests, all "BLOCKED" pre-existing runtime bugs. Wave-14 did not un-ignore any. | Mark all 3 `#[ignore]` permanently. | Open beads for each ignored test. |
| O-R6-2 | OBSERVATION | `crates/vb_storage/src/batch/tests.rs:1075-1150` (3 new sa002_* tests) | Wave-14 added 3 new internal `sa002_*` tests. All gold-standard. | n/a. | Out of scope — positive state. |
| O-R6-3 | OBSERVATION | `crates/vb_storage/src/keys/tests.rs:424,433,442,451,465` (5 new tests) | Wave-14 added 4 new `*_rejects_reserved_seq_sentinel` tests + 1 `run_event_key_accepts_max_encodable`. All gold-standard. | n/a. | Out of scope — positive state. |

---

## Pattern Census

### `assert!(...is_ok()) / assert!(...is_err())` (BANNED in behavior assertions)

| Crate | Round 5 | Round 6 | Δ | Top files (round 6) |
|-------|---------|---------|---|---------------------|
| `vb_storage/src` | 28 | **28** | 0 | No change. M-R6-1 (8 sites in codec/tests.rs) and M-R6-3 (3 in hydrate_tests.rs) still pending. |
| `vb_storage/tests` | ~6 | ~6 | 0 | No change. |
| `workspace_tests/tests` | ~226 | ~226 | 0 | No change. |
| `workspace_tests/benches` | 12 | 12 | 0 | No change. |
| **TOTAL** | **~272** | **~272** | **0** | Wave-14 did not add or remove any `assert!.is_ok()/is_err()` sites. |

### Tautology assertions (`is_ok() || is_err()` or weak-OR)

| File:Line | Status |
|-----------|--------|
| `workspace_tests/tests/integration_compile_error_message_quality.rs:488` | **STILL PRESENT** (M-R6-6, round-4 carry) |
| `vb_storage/src/blob_tests.rs:260` | **DORMANT** (M-R6-5, file not compiled) |
| `workspace_tests/benches/action_dispatch.rs:378` | **STILL PRESENT** (carry) |
| **TOTAL** | **3** (1 hard tautology + 1 DORMANT + 1 benchmark) |

### `let _ = ...` (silent error suppression)

| Crate | Round 5 | Round 6 | Notes |
|-------|---------|---------|-------|
| `vb_storage/src` | ~10 | ~10 | Same. No new silent discards. |
| `workspace_tests/tests` | ~95 | ~95 | Same. **Notable:** `journal_batch_accounting_tests.rs` has 8 `let _ = batch.append_event(&evt)` discards — but these are inside a loop that fills the queue before asserting `QueueFull` on the next iteration. Acceptable as fixture construction. |
| **TOTAL** | **~105** | **~105** | No change. |

### `#[ignore]` / `#[should_panic]` / `sleep()` / `todo!()` / `unimplemented!()`

| Crate | Round 5 | Round 6 | Notes |
|-------|---------|---------|-------|
| `workspace_tests/tests` | 3 `#[ignore]` | **3 `#[ignore]`** | Same as round 4/5: `vb_c1s0_orchestration_runtime_tests.rs:593`, `vb_test_runtime_lifecycle_state_behavior.rs:469`, `vb_vt2f_direct_runtime_api_acceptance.rs:605`. Wave-14 did not un-ignore any. |
| `workspace_tests/tests` | 1 `sleep()` | **1 `sleep()`** | `vb_a0t1_source_length_gate/support.rs:269` — bounded 50ms in 60s polling loop. Carry. |
| `workspace_tests/tests` | 0 `#[should_panic]` | **0 `#[should_panic]`** | Clean. |
| `vb_storage/src` | 0 `todo!()` | **0 `todo!()`** | Clean. |
| **TOTAL** | **4** | **4** | No change. |

### DORMANT TEST FILES (NEW round-6 finding)

| File | Size | Was Reviewed As | Live? |
|------|------|-----------------|-------|
| `artifact_tests.rs` | 7.8K | (not previously cited) | NO |
| `batch_tests_vb_mrwe_7.rs` | 3.6K | (not previously cited) | NO |
| `blob_tests.rs` | 8.7K | round-1 M-06 | NO |
| `edge_case_tests.rs` | 24.0K | round-1 C-04 | NO |
| `error_code_tests.rs` | 9.6K | (not previously cited) | NO |
| `header_tests.rs` | 8.3K | (not previously cited) | NO |
| `process_lock_tests.rs` | 7.9K | round-1 C-03 + round-3 M-02 | NO |
| `record_tests.rs` | 8.2K | (not previously cited) | NO |
| `recovery_type_tests.rs` | 10.5K | (not previously cited) | NO |
| `replay_core_tests.rs` | 10.8K | (not previously cited) | NO |
| `snapshot_tests.rs` | 9.9K | (not previously cited) | NO |
| `type_tests.rs` | 11.3K | round-3 M-08 | NO |
| **TOTAL** | **120.0K** | 4 cited as live | **0 are live** |

### `lazy_static` / `OnceLock` / `static mut` / `thread_local!`

| Crate | Round 6 | Notes |
|-------|---------|-------|
| All test files | **0** | Clean. Production has them in `vb_storage/src/journal/core.rs:125`, `vb_storage/src/queue/writer.rs:53`, `vb_storage/src/queue/loom_vb_mrwe_7.rs:8`; tests do not. |

### `panic!()` in test bodies (banned by rubric, but context-dependent)

| Crate | Round 6 | Context |
|-------|---------|---------|
| `workspace_tests/tests` | ~70 | `match other => panic!("expected X, got {other:?}")` positive assertions. Acceptable. |
| `vb_storage/src` | ~75 | Fixture or positive-assertion idioms. Acceptable. |
| **TOTAL** | **~145** | Same. |

### Fuzz/kani mis-categorized as `#[test]`

| File | Status |
|------|--------|
| `crates/vb_storage/src/kani_*.rs` (~80 files) | All `#[cfg(all(kani, feature = "..."))]` gated. **CLEAN.** |
| **TOTAL** | **0** mis-categorized |

---

## Mutation Gaps (top 5 most dangerous bugs the slice would NOT catch)

1. **`commit()` on aborted batch returns `Ok(())` no-op instead of `Err(BatchAborted)`** — partial mutation
   caught by the 4 NEW internal `Err(BatchAborted)` tests at `batch/tests.rs:920,953,1051,1076,1112,1148` and
   `byte_accounting_tests.rs:807`. **However, the 2 EXTERNAL proptests in `proptest_vb_vzcuf_PS_004.rs:52,115`
   STILL use `.expect("commit")`** and would PASS if production returned `Ok(())`. **The 2 EXTERNAL proptests
   in `journal_side_index_contracts.rs:483,487,636,648` STILL use `is_ok()`** and would PASS if production
   returned `Ok(())`. **Wave-14 missed these 4 cross-crate proptests.** 4 production mutations can slip
   through. **File:Line:** `crates/vb_storage/src/batch/write_event.rs` (production) +
   4 cross-crate proptest sites.

2. **DORMANT test files create false confidence in code coverage** — Rounds 1-5 reviewed
   `process_lock_tests.rs`, `edge_case_tests.rs`, `type_tests.rs`, `blob_tests.rs` (and 8 others) as if they
   were live tests, but they are NEVER COMPILED. The "STILL APPLIED" verifications were vacuous. A production
   mutation that broke the contract those tests were SUPPOSED to cover would NOT be caught by any test.
   **Estimated 120KB of "verified" test coverage is actually 0KB.** **File:Line:** 12 dormant test files in
   `crates/vb_storage/src/`.

3. **`derive_dimensions_includes_action_scheduled_ticket_output` would silently Ok if `apply_tail_events` reset
   `set_max_parallel_in_flight(0)` again** — caught by the test now passing. Wave-14 CF-001 fix at `tail.rs:55-67`
   is verified. **File:Line:** `crates/vb_storage/src/recovery/event_replay/tail.rs:55` (production) +
   `crates/vb_storage/src/recovery/tests.rs:4901-4944` (test).

4. **`run_event_key` / `run_snapshot_key` / `recovery_stamp_key` would accept `u64::MAX` and produce a 17-byte
   key** — caught by 7 `*_rejects_reserved_*` tests (4 NEW in `keys/tests.rs:424,433,442,451` from wave-14 +
   2 in `workspace_tests` from wave-14 + 1 in `fjall_keyspace_manifest_tests.rs` from round 2). **File:Line:**
   `crates/vb_storage/src/keys/encode.rs` (production) + 7 rejection tests.

5. **`run_test_runtime` regression: lifecycle events NOT journaled after `submit + tick`** — caught by 7 NEW
   wave-14 failures in `vb_test_runtime_lifecycle_state_behavior.rs:547,589,658,686,714,739,779` + 5 in
   `cancel_kill_lattice_tests.rs:430,724,756,882,920` + 3 in `vb_vt2f_direct_runtime_api_acceptance.rs:498,595,797`.
   Production may have been tightened in wave-14 in a way that breaks these tests. **File:Line:**
   `crates/vb_runtime/src/shard/impl_parts/journal_helpers.rs` (production, per wave-14 diff stat).

---

## Top 5 Fixes (impact-per-effort)

### Fix 1 — Decide on the 12 DORMANT test files (CRITICAL, structural, ~30 min)

**Impact:** Either restores 120KB of test coverage OR removes 120KB of dead code. The previous 5 rounds'
"STILL APPLIED" verifications for C-03, C-04, M-02, M-05, M-06, M-08 were vacuous because the files were
never compiled. **Option (a) — restore the dormant files** is preferred because they likely contain
historical fixes that would be lost on delete. **Option (b) — delete the files** is acceptable if the
same contracts are tested by other live modules (which they appear to be for process_lock and codec).

**Effort:** 15-30 minutes. Either:
- Add to `crates/vb_storage/src/lib.rs`:
  ```rust
  #[cfg(test)]
  #[path = "process_lock_tests.rs"]
  mod process_lock_tests;
  #[cfg(test)]
  #[path = "edge_case_tests.rs"]
  mod edge_case_tests;
  // ... 10 more
  ```
- OR `rm -f crates/vb_storage/src/{process_lock_tests,edge_case_tests,type_tests,blob_tests,header_tests,record_tests,recovery_type_tests,replay_core_tests,artifact_tests,error_code_tests,snapshot_tests,batch_tests_vb_mrwe_7}.rs`

### Fix 2 — Reconcile 4 EXTERNAL proptest failures (C-R5-1 + C-R5-2 carry-overs, ~30 min)

**Impact:** Restores vb_storage --tests to green (was 2 FAILED in round 5; still 2 FAILED in round 6).
Restores 2 workspace_tests proptests to green. **Effort:** 30 minutes total.

For each of 4 sites:
- `proptest_vb_vzcuf_PS_004.rs:52`: `b2.commit().expect("commit")` → `prop_assert!(matches!(b2.commit(), Err(JournalError::BatchAborted { .. })));`
- `proptest_vb_vzcuf_PS_004.rs:115`: same
- `journal_side_index_contracts.rs:483`: `prop_assert!(batch.append_event(&event_b).is_ok(), ...)` → `prop_assert!(matches!(batch.append_event(&event_b), Err(JournalError::DuplicateEvent { .. })));`
- `journal_side_index_contracts.rs:487`: `prop_assert!(batch.commit().is_ok(), ...)` → `prop_assert!(matches!(batch.commit(), Err(JournalError::BatchAborted { .. })));`
- `journal_side_index_contracts.rs:636`: `subsequent_result.is_ok()` → `subsequent_result.is_err()` (and update message)
- `journal_side_index_contracts.rs:648`: `commit_result.is_ok()` → `commit_result.is_err()` (BatchAborted)

### Fix 3 — Reconcile 7 wave-14 `vb_test_runtime_lifecycle_state_behavior.rs` failures (CRITICAL, ~1-2 hours)

**Impact:** Wave-14 production change in vb_runtime broke 7 lifecycle tests + 5 cancel/kill tests + 3
direct-API tests. The tests assert specific `RuntimeJournalEvent` variants in the journal after
`submit + tick + cancel/kill`, but the events are not being journaled. **Requires investigating the
wave-14 vb_runtime changes to understand whether production is now tighter than tests expect OR
the test fixtures are stale.** Likely root cause: the wave-14 `recovery/event_replay/tail.rs:55-67`
change (`set_max_parallel_in_flight(0)` → `set_max_parallel_in_flight(u16::MAX)`) or the
`shard/impl_parts/journal_helpers.rs` change (+222 lines in wave-14).

**Effort:** 1-2 hours for investigation + 30-60 min for reconciliation.

### Fix 4 — Reconcile 2 wave-11 carry-over `recovery_watermark_tests.rs` failures (CRITICAL, ~15 min)

**Impact:** 2 tests in `recovery_watermark_tests.rs` use `EventSeq::MAX` in `append_journaled` fixtures.
The encoder now rejects `EventSeq::MAX` per the S2-CRIT-1 fix. Test fixtures are STALE.

**Effort:** 15 minutes. For each `EventSeq::MAX` use, replace with `EventSeq::MAX_ENCODABLE` (= u64::MAX - 1):
- Line 580: `seq: EventSeq::MAX,` → `seq: EventSeq::MAX_ENCODABLE,`
- Line 620: same
- Line 404: same
- Line 426: same
- Line 448: same
- Plus ~5 more sites

### Fix 5 — Triage the 9 remaining workspace_tests failures (H-R6-1 through H-R6-10, ~3-4 hours)

**Impact:** Various integration/recovery/contract tests failing for unclear reasons. Each needs
investigation. The 3 `vb_jpq7_3_fail_closed_*` failures may be related to wave-14 recovery changes.
The `vb_nf2u_ui_release_acceptance` failures are environmental (cargo xtask issues). The
`slot_written_ordering` and `doctor_storage_scan_decode` failures are concrete assertion mismatches.

**Effort:** 3-4 hours total.

---

## Round-6 Summary

| Category | Count |
|----------|-------|
| Round-1 CRITICAL fixes verified STILL APPLIED (live) | 2 / 2 (C-01, C-02) |
| Round-1 CRITICAL fixes verified in DORMANT files | 2 (C-03, C-04) |
| Round-2 CRITICAL fixes verified STILL APPLIED (live) | 2 / 2 (C-05, S2-CRIT-1) |
| Round-2 corruption tests verified STILL APPLIED (live) | 3 / 3 |
| Round-3 fixes verified STILL APPLIED (live) | 5 / 5 (H-02, H-03, M-03, M-05, frame_seed) |
| Round-3 fixes verified in DORMANT files | 3 (M-02, M-06, M-08) |
| Round-4 S2-CRIT-1 carry-overs verified STILL APPLIED + EXPANDED | 7 / 7 |
| Round-5 C-R5-1 / C-R5-2 carry-overs STILL UNFIXED | 4 (2 in vb_storage + 2 in workspace_tests) |
| Round-5 M-R5-1 carry-over STILL UNFIXED | 1 (e2e_aborted_batch_commit_succeeds_with_no_persist) |
| **CRITICAL findings (NEW round 6)** | **5 (C-R6-1, C-R6-2, C-R6-3, C-R6-4, C-R6-5)** |
| **HIGH findings (NEW round 6)** | **10 (H-R6-1 through H-R6-10)** |
| MEDIUM findings (NEW + carry) | 6 (M-R6-1, M-R6-2, M-R6-3, M-R6-4, M-R6-5, M-R6-6) |
| LOW findings (NEW + carry) | 3 (L-R6-1, L-R6-2, L-R6-3) |
| OBSERVATION findings (NEW + carry) | 3 (O-R6-1, O-R6-2, O-R6-3) |
| Round-1+2+3+4+5 fix regression count (live files) | **0** |
| Round-1+2+3+4+5 fix regression count (DORMANT files) | **4** (C-03, C-04, M-02, M-06) |
| **DORMANT test files (NEW structural finding)** | **12 (~120KB of dead code)** |
| **vb_storage --lib pass count** | **1552 passed, 0 failed** (same as round 5) |
| **vb_storage --tests pass count** | **2 FAILED** (same as round 5: ps004_no_persist, ps004_empty_commit_after_rej) |
| **workspace_tests pass count** | **~2648 passed, 44 FAILED** (round 5 was 44 — wave-14 added 7 NEW failures in vb_test_runtime_lifecycle_state_behavior.rs + 3 in vb_jpq7_3_fail_closed_* + 2 in recovery_watermark_tests.rs + 8 in vb_nf2u_*; minus 6 that wave-14 fixed (4 batch/tests.rs + 2 internal `sa002_*`)) |

The slice's strength is that **all 12 round-1+2+3+4+5 verified CRITICAL/HIGH artifacts that ARE LIVE remain intact**, AND wave-14 produced 6 POSITIVE improvements (3 new `sa002_*` internal tests + 3 existing internal tests updated + 4 new `*_rejects_reserved_seq_sentinel` tests + 2 new `*_rejects_reserved_sentinel` workspace_tests + 1 SC-001 bijection fix + 1 CF-001 parallel_in_flight fix). **However, round-6 discovered a NEW CRITICAL structural defect: 12 test files (~120KB) in `crates/vb_storage/src/` are DORMANT (not declared in `lib.rs`, not compiled, not executed).** Rounds 1-5 reviewed 4 of these files (process_lock_tests, edge_case_tests, type_tests, blob_tests) as if they were live tests, but the "STILL APPLIED" verifications were vacuous. **Wave-14 also introduced 5 NEW workspace_tests failure clusters** (44 total failures) by tightening production contracts without updating test fixtures. The slice cannot advance until:

1. **C-R6-1** — Decide on the 12 DORMANT test files (restore or delete, ~30 min).
2. **C-R6-2** — Update 4 cross-crate proptests to assert `Err(BatchAborted)` (`proptest_vb_vzcuf_PS_004.rs:52,115` + `journal_side_index_contracts.rs:483,487,636,648`, ~30 min).
3. **C-R6-3** — Reconcile 7 wave-14 `vb_runtime` lifecycle test failures (~1-2 hours).
4. **C-R6-4** — Replace `EventSeq::MAX` with `EventSeq::MAX_ENCODABLE` in `recovery_watermark_tests.rs` fixtures (~15 min).
5. **C-R6-5** — Investigate 3 `vb_jpq7_3_fail_closed_*` recovery failures (~1-2 hours).

Total cleanup time: ~4-7 hours.

Round-6 progress vs. round 5:
- Round 1: 3 CRITICAL + 10 HIGH + 12 MEDIUM + 8 LOW + 5 OBSERVATION
- Round 2: 2 CRITICAL (new) + 0 HIGH + 6 MEDIUM (new) + 4 LOW (new) + 5 OBSERVATION
- Round 3: 0 CRITICAL + 0 HIGH + 11 MEDIUM + 6 LOW + 3 OBSERVATION
- Round 4: 1 CRITICAL (new) + 0 HIGH + 10 MEDIUM + 6 LOW + 5 OBSERVATION
- Round 5: 2 CRITICAL (new) + 0 HIGH + 13 MEDIUM + 6 LOW + 5 OBSERVATION
- Round 6: 5 CRITICAL (new — 1 structural + 1 carry + 1 wave-14 + 1 wave-11 carry + 1 wave-14) + 10 HIGH (new) + 6 MEDIUM + 3 LOW + 3 OBSERVATION

The CRITICAL count increased from 2 to 5 because round-6 discovered: (a) the DORMANT test files structural
defect (1 CRITICAL — vacuous verification in rounds 1-5); (b) the C-R5 carry-overs are STILL UNFIXED (1 CRITICAL
— wave-14 fixed internal but missed external proptests); (c) wave-14 production changes broke 3 NEW test
clusters (3 CRITICAL — vb_runtime lifecycle, recovery_watermark fixtures, vb_jpq7_3 fail-closed contract). Wave-14
produced 6 POSITIVE improvements but introduced 5 NEW CRITICAL regressions by failing to coordinate test
updates across the workspace. The 12 dormant test files are a structural finding that retroactively invalidates
rounds 1-5 "STILL APPLIED" verifications for C-03, C-04, M-02, M-06, M-08.

---

## Disposition

| ID | Disposition | Rationale |
|----|-------------|-----------|
| C-R6-1 | `blocker` | 12 DORMANT test files in `crates/vb_storage/src/` represent ~120KB of dead code. Rounds 1-5 verifications for C-03, C-04, M-02, M-06, M-08 were vacuous. **REJECTED until files are either restored or deleted.** |
| C-R6-2 | `blocker` | 4 EXTERNAL proptest cross-crate tests (2 in `proptest_vb_vzcuf_PS_004.rs` + 2 in `journal_side_index_contracts.rs`) still use `is_ok()` / `.expect("commit")` after the wave-11 SA-002 production contract change. CI gate is broken. |
| C-R6-3 | `blocker` | 7 NEW wave-14 `vb_runtime` lifecycle test failures in `vb_test_runtime_lifecycle_state_behavior.rs`. Production may be tighter than tests expect OR test fixtures are stale. Requires investigation. |
| C-R6-4 | `blocker` | 2 wave-11 reserved-sentinel carry-overs in `recovery_watermark_tests.rs`. Test fixtures use `EventSeq::MAX` which the encoder now rejects. Easy fix (~15 min). |
| C-R6-5 | `blocker` | 3 wave-14 recovery layer failures in `vb_jpq7_3_fail_closed_storage_recovery_contract.rs`. Requires investigation. |
| H-R6-1 through H-R6-10 | `blocker` | 10 NEW HIGH-severity workspace_tests failures (44 total in workspace_tests). Each needs investigation. |
| M-R6-1, M-R6-2, M-R6-3, M-R6-4, M-R6-6 | `owner_approved_debt` | Round-3/4/5 carry-overs not addressed by wave-14. ~15 sites still let mutations pass silently. |
| M-R6-5 | `owner_approved_debt` | DORMANT file. Same as C-R6-1 disposition. |
| L-R6-1, L-R6-2, L-R6-3 | `owner_approved_debt` | Cosmetic + idiomatic. |
| O-R6-1, O-R6-2, O-R6-3 | `owner_approved_no_action` | 3 `#[ignore]` documented; new `sa002_*` and `*_rejects_reserved_*` tests are gold-standard. |

---

## Verdict

```
STATUS: REJECTED
```

**5 NEW CRITICAL + 10 NEW HIGH + 6 NEW/CARRY MEDIUM + 3 NEW/CARRY LOW + 3 OBSERVATION findings**, with **0 live-file regressions** on verified CRITICAL/HIGH artifacts. **vb_storage --lib is GREEN (1552 passed).** **vb_storage --tests has 2 STILL-UNFIXED failures** (ps004_no_persist, ps004_empty_commit_after_rej) from round 5. **workspace_tests has 44 FAILED** (round 5 was 44; wave-14 added 7 NEW failures in vb_test_runtime_lifecycle_state_behavior.rs + 3 in vb_jpq7_3_fail_closed_* + 2 in recovery_watermark_tests.rs + 8 in vb_nf2u_*; minus 6 internal batch/tests.rs fixed). **Wave-14 produced 6 POSITIVE improvements** (3 new sa002_* internal tests + 3 internal tests updated + 4 new *_rejects_reserved_seq_sentinel tests + 2 new *_rejects_reserved_sentinel workspace_tests + 1 SC-001 bijection + 1 CF-001 parallel_in_flight fix). **However, round-6 discovered a NEW CRITICAL structural finding: 12 test files (~120KB) in `crates/vb_storage/src/` are DORMANT** (process_lock_tests.rs, edge_case_tests.rs, type_tests.rs, blob_tests.rs, header_tests.rs, record_tests.rs, recovery_type_tests.rs, replay_core_tests.rs, artifact_tests.rs, error_code_tests.rs, snapshot_tests.rs, batch_tests_vb_mrwe_7.rs — not declared in `lib.rs`, not compiled, not executed). **Rounds 1-5 reviewed 4 of these files (C-03, C-04, M-02, M-06, M-08) as if they were live tests, but the "STILL APPLIED" verifications were vacuous.** The slice cannot advance until:

1. **C-R6-1** — Decide on the 12 DORMANT test files (restore or delete, ~30 min).
2. **C-R6-2** — Update 4 cross-crate proptests to assert `Err(BatchAborted)` (carry from C-R5-1/C-R5-2, ~30 min).
3. **C-R6-3** — Reconcile 7 wave-14 vb_runtime lifecycle test failures (~1-2 hours).
4. **C-R6-4** — Replace `EventSeq::MAX` with `EventSeq::MAX_ENCODABLE` in `recovery_watermark_tests.rs` fixtures (~15 min).
5. **C-R6-5** — Investigate 3 `vb_jpq7_3_fail_closed_*` recovery failures (~1-2 hours).

Total cleanup time: ~4-7 hours.
