# Test Review — Slice 2 (Round 5): vb_storage + workspace_tests

**Scope:** 313 test-bearing Rust files (116 in `crates/vb_storage/`; 197 in `crates/workspace_tests/`; plus benches).
Kani harnesses (`kani_*.rs`, ~80 files) are correctly `#[cfg(kani)]`-gated proof artifacts and excluded from
behavior-test review. Round-5 cwd = `274f30749` (state-11 holzman-rust verification + artifacts).
Round-5 is reviewing the wave that introduced new state-11 contract changes (BatchState::Aborted, parallel_in_flight ceiling, key tests).

**Date:** 2026-06-21
**Reviewer:** test-reviewer agent (slice 2 of 4, round 5)
**Round:** 5 of 40 — verify round-1+2+3+4 fixes + find NEW defects from state-11 commit `274f30749`.

## STATUS: REJECTED

Round-1+2+3+4 CRITICAL and HIGH fixes are **STILL APPLIED with 0 regressions** on the 7 verified CRITICAL/HIGH artifacts (S2-CRIT-1: `u64::MAX → u64::MAX - 1` + `*_rejects_reserved_sentinel` tests, C-01/C-02 tautologies in `integration_compile_error_message_quality.rs` and `integration_runtime_storage_fault_tolerance.rs`, C-03 process_lock security contract, C-04 `encode_accepts_zero_length_payload_and_round_trips` rename, C-05/C-06 declared_keyspaces count=11, M-02/M-03/M-05 contracts_production_binding.rs, frame_seed_slot_dimension_overflow_reports_exact_variant). All 7 round-1+2+3+4 critical fix sites verified at the file:line level. `vb_storage --lib` is now GREEN at 1552 passed, 0 failed (round-4 was 1762 --tests passed but did not include --lib). However, **state-11 introduced 5 NEW CRITICAL contract regressions** by tightening the batch-commit contract (`commit()` on aborted batch now returns `Err(JournalError::BatchAborted)` instead of `Ok(())` no-op) without updating 5 dependent tests in `vb_storage/tests/proptest_vb_vzcuf_PS_004.rs` and `workspace_tests/tests/journal_side_index_contracts.rs`. The workspace_tests failure count is now **44 FAILED** (round-4 was 39 FAILED, +5 NEW). State-11 ALSO fixed the `derive_dimensions_includes_action_scheduled_ticket_output` regression (tail.rs:63 changed `set_max_parallel_in_flight(0)` → `set_max_parallel_in_flight(u16::MAX)`) and added 4 new `*_rejects_reserved_seq_sentinel` tests in `keys/tests.rs` — these are positive changes. **However, the round-4 carry-over workspace_tests failures (39) remain unaddressed**, and the 5 NEW state-11 failures add to the queue. Cannot advance to APPROVED until the 5 state-11 test-vs-production mismatches are reconciled.

---

## Round-1 + Round-2 + Round-3 + Round-4 Fix Verification Table

| Round | ID | File:Line | Defect | Status | Evidence |
|-------|----|-----------|--------|--------|----------|
| 1 | C-01 / H-08 | `integration_compile_error_message_quality.rs:374-378,401-405,426-430` | `assert!(result.is_ok() \|\| result.is_err())` tautology (3 sites) | **STILL APPLIED** | Lines 376, 402, 428 still use `matches!(result, Err(CompileErrors(ref errors)) if errors.iter().any(|e| matches!(e, CompileError::DepthLimit/SequenceLimit/ScalarLimit{...})))`. Line 488 STILL has M-10 tautology `assert!(!errors.is_empty() \|\| is_ok)` — separate carry-over. |
| 1 | C-02 | `integration_runtime_storage_fault_tolerance.rs:215-218` | `assert!(result.is_ok() \|\| result.is_err())` tautology | **STILL APPLIED** | Lines 215-218 still use `matches!(result, Err(ref e) if matches!(e, RuntimeError::InvalidRecoveryHydration))`. **0 regressions.** |
| 1 | C-03 | `process_lock_tests.rs:141-179` | "match-all-outcomes" with `_ = other;` and `Ok(_) => {} \| Err(_) => {}` (2 tests) | **STILL APPLIED** | Lines 149-152: `assert!(matches!(result, Err(JournalError::ProcessLockHeld { .. }))`; lines 156-178: `lock_path.exists()` while open, `!lock_path.exists()` after drop, `result.is_ok()` for re-open. |
| 1 | C-04 | `edge_case_tests.rs:547` | Test name `encode_rejects_zero_length_payload_serialization` contradicted `assert!(result.is_ok())` | **STILL APPLIED** | Renamed to `encode_accepts_zero_length_payload_and_round_trips` (line 547). Round-trip: encodes → asserts `!encoded.is_empty()` → decodes → asserts `decoded.bytes == vec![]`, `decoded.digest == [0u8; 32]`, `envelope.kind == RecordKind::Blob`. |
| 2 | C-05 | `workspace_tests/tests/doctor_key_decode_tests.rs:617-635` | `readonly_journal_declared_keyspaces_returns_ten` asserted `len() == 10` | **STILL APPLIED** | Test renamed to `readonly_journal_declared_keyspaces_returns_eleven` (line 618). Line 624 asserts `assert_eq!(spaces.len(), 11, "declared_keyspaces must return exactly 11 entries (10 historical + run_seq_gap from wave-5/6)")`. Lines 627-630 also assert `names.contains(&"run_seq_gap")`. Plus non-empty check. |
| 2 | C-06 | `workspace_tests/tests/fjall_keyspace_manifest_tests.rs:309-356` | `declared_keyspaces_count` asserted `len() == 10` | **STILL APPLIED** | Lines 312-316: `assert_eq!(keyspaces.len(), 11, "declared_keyspaces must return exactly 11 entries (10 historical + run_seq_gap from wave-5/6)")`. Plus sibling test `declared_keyspaces_contains_required_names` (line 320-343). Plus `declared_keyspaces_no_duplicates` (line 346-356). |
| 2 | S2-CRIT-1 | `journal_tail_scan_fallback_tests.rs` + `fjall_keyspace_manifest_tests.rs` — `u64::MAX → u64::MAX - 1` + `*_rejects_reserved_sentinel` tests | Wave-11 production `ReservedSeqSentinel` rejection at u64::MAX boundary broke tests using u64::MAX | **STILL APPLIED** | Lines 290, 293, 295, 303, 304 (fjall_keyspace_manifest): all use `u64::MAX - 1` and `u64::MAX - 2`. Line 309-320: `max_sequence_encoder_rejects_reserved_sentinel` test asserts `matches!(result, Err(vb_storage::JournalError::ReservedSeqSentinel))`. Lines 985-997 (journal_tail_scan_fallback): `run_event_key_rejects_reserved_sentinel` test asserts `matches!(key, Err(JournalError::ReservedSeqSentinel))`. **State-11 ALSO added 4 NEW `_rejects_reserved_seq_sentinel` tests in `vb_storage/src/keys/tests.rs:423,432,441,451`** — improves coverage. **0 regressions.** |
| 2 | artifacts | `vb_storage/src/journal/tests.rs:1889,1931,1976` | 3 corruption tests for `events_for_run` rejection (BadMagic, PayloadDigestMismatch, PostcardDecodeFailed) | **STILL APPLIED + STILL PASSING** | All 3 tests present at lines 1890-1928, 1932-1973, 1977-2007. Each uses `matches!(result, Err(JournalError::BadMagic { .. }))` / `PayloadDigestMismatch` / `PostcardDecodeFailed`. |
| 3 | H-02 | `vb_storage/src/codec/tests.rs:565,2813,2825,2850` | `assert!(result.is_ok())` smoke tests | **STILL APPLIED** | All 4 sites still use `assert_eq!(result, Ok(()), "...")` with explicit unit comparison. Round-4 M-NEW-2 finding (8 NEW sites at 534, 674, 1756, 2459, 2655, 2739, 2863, 2996) still NOT FIXED — see M-R5-1. |
| 3 | H-03 | `vb_storage/src/security_tests.rs:767` | `assert!(result.is_ok())` SECURITY smoke test | **STILL APPLIED** | Line 767-771 still uses `assert_eq!(result, Ok(()), "SECURITY: correct digest must pass verification (and must NOT silently Ok on wrong digest — see rejects_wrong test)")`. |
| 3 | M-02 | `vb_storage/src/process_lock_tests.rs:213-228` | Smoke test + no `lock_path.exists()` check | **STILL APPLIED** | Both tests still have `let lock_path = temp.path().join(".process.lock"); assert!(lock_path.exists(), "...")` AFTER the smoke check. Decorative + concrete pattern intact. |
| 3 | M-03 | `workspace_tests/tests/contracts_production_binding.rs:170-187, 228-244, 292-313, 330-344` | `is_err()` accepts any variant | **STILL APPLIED** | `test_prod_parse_schema_version_invalid` (170-187) uses `.err().unwrap_or_else(...)` + `message.contains("MISSING_SCHEMA_VERSION"\|"INVALID_VERSION")`. `test_prod_parse_contract_kind_invalid` (228-244), `test_prod_compare_semver_invalid_format` (292-313), `test_prod_parse_vet_exit_code_failure` (330-344) all use concrete `.err().unwrap_or_else(...)` + message-substring checks. |
| 3 | M-05 | `workspace_tests/tests/contracts_production_binding.rs:282` | `assert!(parse_vet_exit_code(0).is_ok())` smoke | **STILL APPLIED** | `test_prod_parse_vet_exit_code_success` (line 321-327) still uses `assert_eq!(parse_vet_exit_code(0), Ok(()), "...")`. |
| 3 | artifact | `frame_seed_slot_dimension_overflow_reports_exact_variant` at `vb_storage/src/recovery/replay/summary/tests.rs:482-500` | Bogus "Expected Ok, got" comment + `assert!(result.is_ok())` | **STILL APPLIED + NOW PASSING** | Round-4 wave-12 fix retained: `matches!(result, Err(RecoveryError::FrameDimensionOverflow { run: found }) if found == run)`. Test PASSES in current state. State-11 ALSO fixed `apply_tail_events` (`tail.rs:63` changed `set_max_parallel_in_flight(0)` → `set_max_parallel_in_flight(u16::MAX)` with new CF-001 comment), which fixed the previously failing `derive_dimensions_includes_action_scheduled_ticket_output` test (now also PASSES). |
| 3 | artifacts | 3 `#[ignore]` behavior tests | Round-4 reduced 5 → 3 `#[ignore]` | **STILL APPLIED** | Confirmed 3 `#[ignore]` at `vb_c1s0_orchestration_runtime_tests.rs:593`, `vb_test_runtime_lifecycle_state_behavior.rs:469`, `vb_vt2f_direct_runtime_api_acceptance.rs:605`. All have `BLOCKED: ... InvalidActionCompletion ... pending vb_runtime fix` ignore reasons. **State-11 did not un-ignore any.** |

**Round-1+2+3+4 regression count: 0.** All 7 verified CRITICAL/HIGH artifacts intact, plus state-11 IMPROVED the production behavior of `apply_tail_events` (parallel_in_flight ceiling fix) and added 4 new `*_rejects_reserved_seq_sentinel` tests in `keys/tests.rs`.

---

## State-11 New Behavior Changes Detected

State-11 commit `274f30749` (2026-06-21) modified production contracts in two areas. State-11 properly updated SOME tests but missed OTHERS, leading to NEW regressions:

### A. BatchState::Aborted change — production tightens, some tests miss update

**Production change:** `crates/vb_storage/src/batch/types.rs:14-37` changed `BatchState::Aborted` from unit variant to `{ reason: &'static str }` variant. Consequence: `commit()` on aborted batch now returns `Err(JournalError::BatchAborted { .. })` (was `Ok(())` no-op). State-11 commit `CR-SA-002`.

**Tests UPDATED by state-11 (correctly):**
- `crates/vb_storage/src/batch/tests.rs:919-928` (`batch_append_event_rejects_intra_batch_duplicate`): now asserts `matches!(commit_result, Err(JournalError::BatchAborted { .. }))`.
- `crates/vb_storage/src/batch/tests.rs:951-960` (`batch_append_event_intra_batch_dedup_aborts_batch`): now asserts `Err(BatchAborted)`.
- `crates/vb_storage/src/batch/tests.rs:1049-1056` (new test at line ~1049): asserts `Err(BatchAborted)`.
- `crates/vb_storage/src/batch/byte_accounting_tests.rs:786-809` (`e2e_aborted_batch_commit_succeeds_with_no_persist`): renamed assertions to assert `Err(BatchAborted)` — **but the test NAME still says "succeeds_with_no_persist"** (misleading).
- Plus 2-3 more in `batch/tests.rs:920-1071`.

**Tests NOT UPDATED (regressing):**
- `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs:43-55` (`ps004_no_persist`): still calls `b2.commit().expect("commit")` at line 52 — fails with `BatchAborted { reason: "duplicate_event_committed" }`.
- `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs:106-116` (`ps004_empty_commit_after_rej`): same root cause — `b2.commit().expect("commit")` at line 115 fails.
- `crates/workspace_tests/tests/journal_side_index_contracts.rs:464-487` (`test_duplicate_idempotency_key`): line 487 `prop_assert!(batch.commit().is_ok(), ...)` — fails.
- `crates/workspace_tests/tests/journal_side_index_contracts.rs:590-648` (`test_aborted_gate_blocks_subsequent_staging`): line 635-638 `prop_assert!(subsequent_result.is_ok(), "after abort, append_event must return Ok (not stage)")` — fails because subsequent appends now also return `Err(DuplicateEvent)` (different contract: abort blocks subsequent append_event rather than silently accepting them).
- `crates/workspace_tests/tests/journal_side_index_contracts.rs:648` (`commit on aborted batch must return Ok`) — fails (now returns Err).

### B. parallel_in_flight ceiling fix — state-11 IMPROVEMENT

**Production change:** `crates/vb_storage/src/recovery/event_replay/tail.rs:55-63` changed `frame.set_max_parallel_in_flight(0)` → `frame.set_max_parallel_in_flight(u16::MAX)` with new CF-001 comment explaining the ceiling semantics.

**Consequence:** `apply_tail_events` no longer fails with "parallel_in_flight overflow" when processing `ActionScheduled` or `ActionScheduledTicket` events. The previously-failing test `recovery::tests::hydrate_run_frame_tests::derive_dimensions_includes_action_scheduled_ticket_output` at `crates/vb_storage/src/recovery/tests.rs:4901-4944` NOW PASSES (verified by filtered run). **This is a positive state-11 change.**

### C. New key tests added

`crates/vb_storage/src/keys/tests.rs:423-470` added 4 NEW tests in state-11:
- `run_event_key_rejects_reserved_seq_sentinel` (line 423)
- `run_snapshot_key_rejects_reserved_seq_sentinel` (line 432)
- `recovery_stamp_key_rejects_reserved_seq_sentinel` (line 441)
- `event_seq_try_new_rejects_reserved_sentinel` (line 450)
- `run_event_key_accepts_max_encodable` (line 465)

All use `matches!(result, Err(JournalError::ReservedSeqSentinel))` and `EventSeq::MAX_ENCODABLE` (= u64::MAX - 1). **Positive state-11 change.**

---

## Findings (CRITICAL first)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix |
|----|-----|-----------|--------|------------------------------|------------------|
| **C-R5-1** | **CRITICAL** | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs:43-55` (`ps004_no_persist`) and `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs:106-116` (`ps004_empty_commit_after_rej`) | **NEW Round-5 regression introduced by state-11 commit `274f30749`.** State-11 changed `BatchState::Aborted` (production now has `{ reason: &'static str }` field) AND changed `commit()` on aborted batch to return `Err(JournalError::BatchAborted { .. })`. Internal `batch/tests.rs` and `byte_accounting_tests.rs` were updated to assert `Err(BatchAborted)`, but the external proptest `ps004_no_persist` (line 52: `b2.commit().expect("commit")`) and `ps004_empty_commit_after_rej` (line 115: `b2.commit().expect("commit")`) still use `.expect("commit")` — fail with `BatchAborted { reason: "duplicate_event_committed" }`. | Either revert `commit()` to return `Ok(())` on aborted batch (regression of state-11 SA-002 fix), OR update the tests to assert the new contract. The 2 internal tests `batch_append_event_rejects_intra_batch_duplicate` and `batch_append_event_intra_batch_dedup_aborts_batch` already assert the new contract correctly. | For `ps004_no_persist` line 52: replace `b2.commit().expect("commit")` with `let commit_result = b2.commit(); prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted { .. })), "commit on aborted batch must return Err(BatchAborted), got {commit_result:?}");`. Same for `ps004_empty_commit_after_rej` line 115. |
| **C-R5-2** | **CRITICAL** | `crates/workspace_tests/tests/journal_side_index_contracts.rs:464-498` (`test_duplicate_idempotency_key`) and `:590-648` (`test_aborted_gate_blocks_subsequent_staging`) | **NEW Round-5 regression introduced by state-11.** Same root cause as C-R5-1. `test_duplicate_idempotency_key` line 483: `prop_assert!(batch.append_event(&event_b).is_ok(), "append_event B must succeed")` — fails because state-11 rejects intra-batch duplicate appends with `Err(DuplicateEvent)`. The test was written assuming Fjall last-write-wins + batch accepted all events. `test_aborted_gate_blocks_subsequent_staging` line 635: `prop_assert!(subsequent_result.is_ok(), "after abort, append_event must return Ok (not stage)")` — fails because state-11 rejects subsequent appends with `Err(DuplicateEvent)` (different contract from round-4 expectation that aborted batch is a "safe no-op" with Ok returns). | Same as C-R5-1: revert production contract (rollback state-11 SA-002 fix) OR update tests to assert the new contract. | Update `test_duplicate_idempotency_key` to expect `Err(DuplicateEvent)` on the second append. Update `test_aborted_gate_blocks_subsequent_staging` to expect subsequent appends to also return `Err(DuplicateEvent)` (or some other typed error) AND commit() to return `Err(BatchAborted)`. Consider renaming the test to reflect new contract: e.g., `commit_on_aborted_batch_surfaces_typed_error`. |
| M-R5-1 | MEDIUM | `crates/vb_storage/src/batch/byte_accounting_tests.rs:786` | **NEW Round-5 finding.** Test name `e2e_aborted_batch_commit_succeeds_with_no_persist` is misleading after state-11 changes. Line 787 comment still says `E03: Aborted batch (duplicate) commit succeeds as no-op.` Line 804 comment says `// Commit must surface the abort as a typed error per SA-002.` The TEST NAME contradicts BOTH comments and the new assertions on line 805-809. | n/a — purely cosmetic. | Rename to `e2e_aborted_batch_commit_surfaces_typed_error` and update line 787 comment to `E03: Aborted batch (duplicate) commit surfaces typed BatchAborted error (no persistence).` |
| M-R5-2 | MEDIUM | `crates/vb_storage/src/batch/tests.rs:919-928, 951-960, 1049-1056` | Round-4 carry-over H-02 / Round-3 L-02. These sites previously asserted `expect("aborted commit returns Ok(())")` and were correctly updated in state-11 to `matches!(commit_result, Err(JournalError::BatchAborted { .. }))`. **However, the test name at line 919 `batch_append_event_rejects_intra_batch_duplicate` is fine, but the surrounding `prop_assert` calls** at line 487 in `journal_side_index_contracts.rs` (cross-crate test) still use the OLD contract. See C-R5-2. | n/a — already addressed. | Already addressed in `batch/tests.rs` itself; just need to propagate to cross-crate tests. |
| M-R5-3 | MEDIUM | `crates/vb_storage/src/batch/tests.rs:920-928, 951-960, 1049-1056` | Round-4 carry-over. Internal `batch/tests.rs` correctly updated to assert `Err(BatchAborted)`. **State-11 added 3+ new internal test sites** with the same correct pattern. **Positive state-11 improvement.** | n/a | None — observe as gold-standard pattern. |
| M-01 | MEDIUM | `crates/vb_storage/src/hydrate_tests.rs:281,287,374` (3 sites, carry from round 3 M-01 / round 4 M-01) | `assert!(result.is_ok(), "...")` smoke tests on `validate_tail_first_seq_contiguous_with_snapshot` and `validate_snapshot_recovery_inputs`. State-11 did not address. | Replace `validate_tail_first_seq_contiguous_with_snapshot` with `Ok(())` always. All 3 sites pass. | Convert each to `assert_eq!(result, Ok(()), "...")`. |
| M-04 | MEDIUM | `crates/workspace_tests/tests/vb_test_compile_parse_validate_behavior.rs:184-188` (`parse_rejects_whitespace_only_source`, carry from round-3 M-04 / round-1 H-07) | `assert!(result.is_err(), "whitespace-only source should fail")` — accepts any error. State-11 did not address. | Make `YamlCompiler::parse_ast(b"   \n\n   \n")` return `Err(CompileError::Other)`. Test passes. | Add `let msg = result.unwrap_err().to_string(); assert!(msg.contains("empty") \|\| msg.contains("whitespace"));` |
| M-06 | MEDIUM | `crates/vb_storage/src/blob_tests.rs:246-263` (line 260, carry from round-3 M-06 / round-4 M-06) | `result.is_err() \|\| result.map_or(false, \|opt\| opt.is_none())` — accepts EITHER `Err(anything)` OR `Ok(None)`. State-11 did not address. | Make `journal.blob(digest)` return `Ok(None)` for corrupt data. Test passes silently. SECURITY-relevant. | Replace with `assert!(matches!(result, Err(JournalError::BlobDecode(_) \| JournalError::PayloadDigestMismatch { .. })));` |
| M-07 | MEDIUM | `crates/vb_storage/src/recovery/recovery_unit_tests.rs:796-805` (line 800, carry from round-3 M-07 / round-4 M-07) | `assert!(result.is_err());` immediately followed by `matches!(result.unwrap_err(), RecoveryError::NoRecoveryData { .. })`. First is decorative. | n/a — second assertion is concrete. | Remove redundant `assert!(result.is_err())` on line 800. |
| M-08 | MEDIUM | `crates/vb_storage/src/type_tests.rs:191-197` (line 196, carry from round-3 M-08 / round-4 M-08) | `assert!(err.is_err(), "zero should fail")` — accepts any variant. `JournalBatchSize::try_from_usize(0)` should produce specific `JournalBatchSizeError::Zero`. State-11 modified this file (added 30 lines per diff) but did NOT fix this site. | Make `try_from_usize(0)` return `Err(JournalBatchSizeError::SomeOtherVariant)`. Test passes. | Replace with `assert!(matches!(err, Err(JournalBatchSizeError::Zero)));` |
| M-09 | MEDIUM | `crates/workspace_tests/tests/doctor_storage_scan_decode_tests.rs:1266-1282` (carry from round-3 M-09 / round-4 M-09) | `parse_decode_error_invalid_keyspace_path` uses `assert!(result.is_err(), "expected error opening nonexistent path")` followed by match accepting any Err variant. State-11 did not address. | Make `FjallJournal::open` return `Err(JournalError::ProcessLockIo { ... })` for nonexistent path. Test passes silently. | Replace with `assert!(matches!(result, Err(JournalError::Fjall(_) \| JournalError::PathNotFound { .. })));` |
| M-10 | MEDIUM | `crates/workspace_tests/tests/integration_compile_error_message_quality.rs:479-488` (carry from round-3 M-10 / round-4 M-10) | `assert!(!errors.is_empty() \|\| is_ok)` NEW TAUTOLOGY from wave-7/8 still NOT FIXED. State-11 did not address. | Delete the version validation in production (return `Ok(_)` for any version). Test passes (`is_ok == true`). | Replace with concrete contract: `assert!(matches!(result, Err(CompileErrors(ref errors)) if errors.iter().any(|e| matches!(e, CompileError::InvalidVersion { .. }))));` |
| M-NEW-2 | MEDIUM | `crates/vb_storage/src/codec/tests.rs:534,674,1756,2459,2655,2739,2863,2996` (8 sites — round-4 carry) | `assert!(result.is_ok(), "...")` smoke tests on codec accept/decode functions. Round-1 H-02 fixed 4 sites but missed these 8. State-11 did not address. | Replace each function with `Ok(())` for any input. All 8 tests pass. | Convert each to `assert_eq!(result, Ok(()), "...")`. |
| L-01 | LOW | `crates/vb_storage/src/tests/chunk_012.rs:181` (carry from round-3 L-01 / round-4 L-01) | Test name `declared_keyspaces_returns_ten_entries` says "ten" but asserts `keyspaces.len() == 11` on line 186. Comment on line 184 says "exactly 11". Stale name. | n/a | Rename to `declared_keyspaces_returns_eleven_entries`. |
| L-02 | LOW | `crates/vb_storage/src/codec/tests/replay_integrity.rs:201` (carry from round-4 L-02) | `assert!(result.is_ok(), "next_seq(u64::MAX-1) must succeed")` — smoke test on `next_seq` boundary. | Replace `next_seq(u64::MAX-1)` with `Ok(EventSeq::new(u64::MAX))` (skip the +1). Test passes. | Convert to `assert_eq!(result, Ok(EventSeq::new(u64::MAX)), "next_seq at boundary")` and add `assert_eq!(result.unwrap().get(), u64::MAX)`. |
| L-03 | LOW | `crates/vb_storage/src/tests/chunk_004.rs:166,274`, `chunk_008.rs:173,181,185`, `chunk_040.rs:212,227,268` (8 sites, carry from round-1 H-05 / round-3 L-03 / round-4 L-03) | `assert!(journal.is_ok(), "...")` smoke tests on `FjallJournal::open`/`open_store`/`init_keyspaces`. Decorative + concrete. | n/a — concrete round-trip catches production mutations. | Acceptable as decorative + concrete. |
| L-04 | LOW | `crates/workspace_tests/tests/doctor_storage_scan_decode_tests.rs:1363-1364,1457-1460` (4 sites, carry from round-3 L-04 / round-4 L-04) | `assert!(header_result.is_ok()); assert!(full_result.is_ok());` at lines 1363-1364 (decorative); `assert!(decoded_header.is_ok())` at 1457-1460 (decorative). | For 1457: make `decode_record_header` always return `Ok(RecordHeader::default())`. Test passes. | For line 1457 add `assert_eq!(decoded_header.unwrap().sequence, 1)`. |
| L-05 | LOW | `crates/workspace_tests/tests/vb_test_core_workflow_slot_behavior.rs:694,730,810,997,1104,1117` (6 sites, carry from round-3 L-05 / round-4 L-05) | `assert!(result.is_err());` followed by `let err = result.unwrap_err(); assert!(matches!(err, CoreError::SlotUninitialized { slot } if slot == SlotIdx::new(0)));`. Decorative + concrete. | n/a | Acceptable. |
| L-06 | LOW | `crates/vb_storage/src/security_tests.rs:1047-1063` (carry from round-3 L-06 / round-4 L-06) | `assert!(result.is_ok(), "re-open after drop must succeed because lock was released")` — happy path. | n/a | Add `assert!(!lock_path.exists(), ".process.lock must be released on drop");`. |
| O-01 | OBSERVATION | `crates/workspace_tests/tests/vb_test_runtime_lifecycle_state_behavior.rs:469`, `vb_vt2f_direct_runtime_api_acceptance.rs:605`, `vb_c1s0_orchestration_runtime_tests.rs:593` (3 sites — same as round 4) | `#[ignore]` on behavior tests, all "BLOCKED" pre-existing runtime bugs. State-11 did not un-ignore any. | Mark all 3 `#[ignore]` permanently. | Open beads for each ignored test to track closure. |
| O-02 | OBSERVATION | `crates/workspace_tests/tests/runtime_version_barrier_tests.rs:314,340,366,415,436,457,478,499,520,547,612,668` (12 sites — round-4 carry) | All 12 sites use **decorative + concrete**: `assert!(result.is_err());` followed by `let err = result.unwrap_err(); assert!(matches!(err, AdmissionError::ArtifactInvalidGateCount { found, required } if found == X && required == 15))`. | n/a | Acceptable as gold-standard idiom. |
| O-03 | OBSERVATION | `crates/workspace_tests/tests/vb_test_cli_storage_io_behavior.rs:356,401,...` (17 sites, round-4 carry) | `assert!(result.is_ok(), "events_for_run must succeed")` followed by `let events = result.expect("events ok")` and concrete `matches!(terminal, Some(JournalEvent::RunFinished { .. }))` checks. Decorative + concrete. | n/a | Acceptable. |
| O-04 | OBSERVATION | `crates/workspace_tests/benches/action_dispatch.rs:378` (carry from round-1 O-01 / round-3 O-03 / round-4 O-04) | `assert!(result.is_err() \|\| result.is_ok(), "dispatch of unknown action")` — explicit tautology in a benchmark. | n/a — benchmarks are not behavior tests. | Acceptable. |
| O-05 | OBSERVATION | `crates/vb_storage/src/security_tests.rs:1076` (carry from round-3 / round-4 O-05) | `assert!(result.is_err(), "second open must fail")` followed by `assert_eq!(before_count, after_count, "no new files should be created when lock acquisition fails")`. Decorative + concrete. | n/a | Acceptable. |

---

## Pattern Census

### `assert!(...is_ok()) / assert!(...is_err())` (BANNED in behavior assertions)

| Crate | Round 4 | Round 5 | Δ | Top files (round 5) |
|-------|---------|---------|---|---------------------|
| `vb_storage/src` | ~30 | **28** | -2 | `hydrate_tests.rs` (3 — carry M-01), `tests/chunk_040.rs` (3 — carry), `tests/chunk_008.rs` (3 — carry), `tests/chunk_004.rs` (2 — carry), `process_lock_tests.rs` (2 — decorative + concrete), `codec/tests.rs` (**8 NEW**: 534, 674, 1756, 2459, 2655, 2739, 2863, 2996 — round-4 M-NEW-2 carry), `codec/tests/replay_integrity.rs` (1 — L-02 carry), `recovery/recovery_unit_tests.rs` (1 — M-07 carry), `recovery/replay/summary/tests.rs` (1 — fixed), `type_tests.rs` (1 — M-08 carry), `security_tests.rs` (1 — O-05), `recovery/property_tests/error_recovery.rs` (1 — `prop_assert`), `property_tests/proptest_digest_stability.rs` (2), `property_tests/proptest_taint_safety.rs` (4), `property_tests/proptest_bound_enforcement.rs` (1), `property_tests/proptest_layout_stability.rs` (1) |
| `vb_storage/tests` | ~6 | ~6 | 0 | `vb_core_atomic_admission_red.rs` (1 — round-4 M-NEW-3 carry), `vb_god2f_classification_properties.rs` (2), `vb_god2f_recovery_properties.rs` (2), `proptest_journal_idempotency.rs` (1) |
| `workspace_tests/tests` | ~135 | ~110 (round-5 measure: `assert!.*\.is_ok()`) + ~116 (`assert!.*\.is_err()`) = **~226** | +91 (rough re-count, includes prop_assert) | `integration_validate_yaml_parsing.rs`, `runtime_version_barrier_tests.rs` (12 — acceptable O-02), `contracts_production_binding.rs` (4 — all FIXED), `vb_test_core_workflow_slot_behavior.rs` (~15 — acceptable O-03/L-05), `vb_test_cli_storage_io_behavior.rs` (17 — acceptable O-03), `doctor_storage_scan_decode_tests.rs` (13 — mostly decorative), `integration_compile_error_message_quality.rs` (3 — including M-10 carry), `vb_8mdp_7_resource_admission_props.rs` (10, prop_assert), `vb_test_compile_parse_validate_behavior.rs` (9 — including M-04 carry), `integration_validate_policy_enforcement.rs` (5), `vb_qi37_2_4_integration_budget_errors.rs` (5), `vb_eepg_bdd_tests.rs` (3) |
| `workspace_tests/benches` | 12 | 12 | 0 | `action_dispatch.rs:378` — explicit tautology. Acceptable. |
| **TOTAL** | **~183** | **~272** | **+89** | The increase is from a more accurate round-5 re-grep using `rg -n "assert!.*\.is_ok\(\)"` and `rg -n "assert!.*\.is_err\(\)"` separately (vs round-4's combined count). State-11 did not add new `assert!.*is_ok()/is_err()` smoke sites, but did NOT remove carry-overs either. |

### Tautology assertions (`is_ok() || is_err()` or weak-OR)

| File:Line | Status |
|-----------|--------|
| All 4 round-1 tautology sites (C-01, C-02) | **DELETED** (round-1 fix verified) |
| `crates/vb_storage/src/blob_tests.rs:260` | **STILL PRESENT** — `result.is_err() \|\| result.map_or(false, \|opt\| opt.is_none())` weak-OR. Re-classified MEDIUM (M-06). |
| `crates/workspace_tests/tests/integration_compile_error_message_quality.rs:488` | **STILL PRESENT** — `assert!(!errors.is_empty() \|\| is_ok)` tautology. Re-classified MEDIUM (M-10). |
| `crates/vb_storage/tests/vb_core_atomic_admission_red.rs:1151` | **STILL PRESENT** — `prop_assert!(result.is_err() \|\| journal.compiled_ir(...).unwrap().is_none())` weak-OR with hidden `unwrap()`. Re-classified MEDIUM (M-NEW-3 carry). |
| `crates/workspace_tests/benches/action_dispatch.rs:378` | **STILL PRESENT** — benchmark context. Observation-level (O-04). |
| **TOTAL** | **2 hard tautologies (1 NEW + 1 carry) + 1 weak-OR (carry) + 1 benchmark tautology (carry)** |

### `let _ = ...` (silent error suppression)

| Crate | Round 4 | Round 5 | Notes |
|-------|---------|---------|-------|
| `vb_storage/src` | ~10 | ~10 | `kani_*` (3 — proof harnesses), `index_tests.rs` (7 — fixture iteration), `tests/chunk_*.rs` (~10 — `builder.try_push()` discarding capacity check), `proptest_storage.rs` (~5 — proptest input verification), `recovery/recovery_unit_tests.rs` (1 — `_exhaustive_match`). All acceptable as fixture/proof-harness patterns. **No new silent discards introduced by state-11.** |
| `workspace_tests/tests` | ~95 | ~95 | `bdd_validation_tests.rs:1364-1402` (37 — variant-existence check), `timer_deadline_primitive_tests.rs` (~50 — `wheel.insert()` setup), `bdd_idempotency.rs` (2), `cancel_kill_lattice_tests.rs` (2). All acceptable. |
| **TOTAL** | **~105** | **~105** | Same as round 4. **No new silent discards introduced.** |

### `#[ignore]` / `#[should_panic]` / `sleep(` / `todo!()` / `unimplemented!()`

| Crate | Round 4 | Round 5 | Notes |
|-------|---------|---------|-------|
| `workspace_tests/tests` | 3 `#[ignore]` | **3 `#[ignore]`** | **Same as round 4**: `vb_c1s0_orchestration_runtime_tests.rs:593`, `vb_test_runtime_lifecycle_state_behavior.rs:469`, `vb_vt2f_direct_runtime_api_acceptance.rs:605`. State-11 did not un-ignore any. See O-01. |
| `workspace_tests/tests` | 1 `sleep()` | **1 `sleep()`** | `vb_a0t1_source_length_gate/support.rs:269` — bounded 50ms in 60s polling loop. **However, 2 tests in this file are FAILING** (`test_out_of_scope_vb_cli_xtask_changes_are_routed_with_touched_package_evidence` and `test_full_source_length_pipeline`). See C-NEW-1 (carry). |
| `workspace_tests/tests` | 0 `#[should_panic]` | **0 `#[should_panic]`** | Clean. |
| `vb_storage/src` | 0 `todo!()` / `unimplemented!()` | **0 `todo!()` / `unimplemented!()`** | Clean. |
| **TOTAL** | **4** | **4** | Same as round 4. **No new `#[ignore]` introduced.** |

### `lazy_static` / `OnceCell` / `static mut` / `thread_local!`

| Crate | Round 5 | Notes |
|-------|---------|-------|
| All test files | **0** | Clean. Production has them in `vb_storage/src/journal/core.rs:125`, `vb_storage/src/queue/writer.rs:53`, `vb_storage/src/queue/loom_vb_mrwe_7.rs:8`; tests do not. |

### `panic!()` in test bodies (banned by rubric, but context-dependent)

| Crate | Round 4 | Round 5 | Context |
|-------|---------|---------|---------|
| `workspace_tests/tests` | ~70 | ~70 | `match other => panic!("expected X, got {other:?}")` positive assertions. Acceptable. |
| `vb_storage/src` | ~75 | ~75 | Fixture or positive-assertion idioms. Acceptable. |
| **TOTAL** | **~145** | **~145** | Same as round 4. |

### `.ok();` / `.err();` silent Result conversion

| Crate | Round 5 | Notes |
|-------|---------|-------|
| `workspace_tests/tests` | 11 | `vb_8mdp_7_resource_admission_props.rs` (11 sites) — silent discard of `enqueue`/`tick` errors. Same as round 4. |
| `vb_storage/src/proptests.rs` | 2 | Same as round 4. |
| **TOTAL** | **13** | Unchanged from round 4. |

### Fuzz/kani mis-categorized as `#[test]`

| File | Status |
|------|--------|
| `crates/vb_storage/src/kani_*.rs` (~80 files) | All `#[cfg(all(kani, feature = "..."))]` gated. **CLEAN.** |
| `crates/fuzz/fuzz_targets/*.rs` | Outside slice 2 scope. **CLEAN.** |
| **TOTAL fuzz/kani mis-categorized** | **0** |

---

## Mutation Gaps (top 5 most dangerous bugs the slice would NOT catch)

1. **`commit()` on aborted batch returns `Ok(())` no-op instead of `Err(BatchAborted)`** — full mutation caught by the 4 NEW internal `Err(BatchAborted)` tests at `batch/tests.rs:920,953,1051` and `byte_accounting_tests.rs:807`. **However, the 5 affected external tests are STILL using the OLD contract (C-R5-1, C-R5-2)**, so a partial mutation that returns `Ok(())` for SOME aborted cases would slip through. **File:Line:** `crates/vb_storage/src/batch/write_event.rs` (production) + 5 affected external tests.

2. **`derive_dimensions_includes_action_scheduled_ticket_output` would silently Ok if `apply_tail_events` reset `set_max_parallel_in_flight(0)` again** — partial mutation caught by the test now passing. State-11 fix `tail.rs:63` `set_max_parallel_in_flight(u16::MAX)` is verified. **File:Line:** `crates/vb_storage/src/recovery/event_replay/tail.rs:63` (production) + `crates/vb_storage/src/recovery/tests.rs:4901-4944` (test).

3. **`run_event_key` / `run_snapshot_key` / `recovery_stamp_key` would accept `u64::MAX` and produce a 17-byte key** — caught by 7 `*_rejects_reserved_seq_sentinel` tests (3 in `workspace_tests/journal_tail_scan_fallback_tests.rs` + 3 in `vb_storage/keys/tests.rs` + 1 in `fjall_keyspace_manifest_tests.rs`). State-11 added 4 NEW tests in `keys/tests.rs:423,432,441,451,465` improving coverage. **File:Line:** `crates/vb_storage/src/keys/encode.rs` (production) + 7 rejection tests.

4. **`events_for_run` silently returns corrupt events instead of typed error** — caught by 3 corruption tests at `journal/tests.rs:1890-1928,1932-1973,1977-2007`. **File:Line:** `crates/vb_storage/src/journal/events.rs` (production) + 3 corruption tests.

5. **`verify_digest_match` returns `Ok(())` for ANY payload** — partial mutation caught by wrong-digest tests. Round-4 verification confirms `codec/tests.rs:565` and `security_tests.rs:767` use `assert_eq!(result, Ok(()), ...)` with security-relevant comments. **However, M-NEW-2 reveals 8 OTHER sites** (`codec/tests.rs:534,674,1756,2459,2655,2739,2863,2996`) where the wrong-input rejection tests are missing or not paired. **File:Line:** `crates/vb_storage/src/codec/tests.rs:534,674,1756,2459,2655,2739,2863,2996` (carry M-NEW-2).

---

## Top 5 Fixes (impact-per-effort)

### Fix 1 — Reconcile 5 NEW state-11 contract regressions (CRITICAL, blocker)

**Impact:** Restores vb_storage --tests + workspace_tests to green. The 5 NEW failures all share the same root cause: state-11 tightened `commit()` on aborted batch to return `Err(BatchAborted)`, but 5 external tests still expect `Ok(())`. **Effort:** 15-30 minutes total.

For each:
- **`ps004_no_persist`** (`vb_storage/tests/proptest_vb_vzcuf_PS_004.rs:43-55` line 52): replace `b2.commit().expect("commit")` with `prop_assert!(matches!(b2.commit(), Err(JournalError::BatchAborted { .. })));`.
- **`ps004_empty_commit_after_rej`** (`vb_storage/tests/proptest_vb_vzcuf_PS_004.rs:106-116` line 115): same.
- **`test_duplicate_idempotency_key`** (`workspace_tests/tests/journal_side_index_contracts.rs:464-498`): line 483 `prop_assert!(batch.append_event(&event_b).is_ok(), ...)` — change to `prop_assert!(matches!(batch.append_event(&event_b), Err(JournalError::DuplicateEvent { .. })));`. Then line 487 `prop_assert!(batch.commit().is_ok(), ...)` — change to `prop_assert!(matches!(batch.commit(), Err(JournalError::BatchAborted { .. })));`. **Update test docstring at line 446 to reflect new contract.**
- **`test_aborted_gate_blocks_subsequent_staging`** (`workspace_tests/tests/journal_side_index_contracts.rs:590-648`): the entire test is based on the old contract that aborted batch is a "safe no-op". Line 635 `prop_assert!(subsequent_result.is_ok(), "after abort, append_event must return Ok (not stage)")` — change to expect `Err(DuplicateEvent)` (or some other typed error). Line 648 `commit on aborted batch must return Ok` — change to `must return Err(BatchAborted)`. **Consider renaming test to `test_aborted_gate_rejects_subsequent_appends` to reflect new contract.**

### Fix 2 — Rename misleading test name in `byte_accounting_tests.rs:786` (MEDIUM, 1 min)

```rust
// BEFORE:
fn e2e_aborted_batch_commit_succeeds_with_no_persist() {
    // E03: Aborted batch (duplicate) commit succeeds as no-op.

// AFTER:
fn e2e_aborted_batch_commit_surfaces_typed_error() {
    // E03: Aborted batch (duplicate) commit surfaces typed BatchAborted error (no persistence).
```

### Fix 3 — Convert 8 carry `assert!(result.is_ok())` in `codec/tests.rs:534,674,1756,2459,2655,2739,2863,2996` to `assert_eq!`

**Impact:** 8 SECURITY/codec-relevant success-path tests gain concrete value assertions. **Effort:** 30 minutes.

### Fix 4 — Convert 3 carry `assert!(result.is_ok())` in `hydrate_tests.rs:281,287,374` to `assert_eq!`

**Impact:** 3 success-path tests gain concrete value assertions. **Effort:** 10 minutes.

### Fix 5 — Fix M-04, M-06, M-07, M-08, M-09, M-10 (MEDIUM, ~45 min)

Tighten ~12 variant checks across 6 carry-over files. Replace each `assert!(err.is_err(), ...)` with concrete `matches!(err, Err(SomeVariant))`.

---

## Round-5 Summary

| Category | Count |
|----------|-------|
| Round-1 CRITICAL fixes verified STILL APPLIED | 4 / 4 |
| Round-2 CRITICAL fixes verified STILL APPLIED | 3 / 3 (count + corruption tests) |
| Round-3 carry-overs STILL APPLIED | 6 of 6 (M-02, M-03, M-05, H-02/H-03, 3 `#[ignore]`, frame_seed_slot_dimension_overflow) |
| Round-4 carry-overs STILL APPLIED | 1 / 1 (S2-CRIT-1 + rejects_reserved_sentinel tests) |
| **CRITICAL findings (NEW round 5)** | **2 (C-R5-1, C-R5-2)** |
| HIGH findings (NEW) | 0 |
| MEDIUM findings (NEW + carry) | 13 (M-R5-1, M-R5-2, M-R5-3 + M-01..M-10 + M-NEW-2) |
| LOW findings (NEW + carry) | 6 (L-01..L-06) |
| OBSERVATION findings (NEW + carry) | 5 (O-01..O-05) |
| Round-1+2+3+4 regression count | **0** |
| State-11 IMPROVEMENTS (positive) | 5 (1 `apply_tail_events` fix at `tail.rs:63` + 4 new `*_rejects_reserved_seq_sentinel` tests in `keys/tests.rs:423,432,441,451,465` + 4 new internal `Err(BatchAborted)` assertions in `batch/tests.rs` + 1 in `byte_accounting_tests.rs`) |
| State-11 NEW REGRESSIONS (negative) | 5 (2 in `vb_storage/tests/proptest_vb_vzcuf_PS_004.rs` + 2 in `workspace_tests/tests/journal_side_index_contracts.rs` + 1 misleading test name in `byte_accounting_tests.rs:786`) |
| **vb_storage --lib pass count** | **1552 passed, 0 failed** (round-4 did not measure --lib; --tests was 1762 passed) |
| **vb_storage --tests pass count** | **many passed, 2 FAILED** (round-4 was 1762 passed, 0 failed — but round-4 missed the 2 proptest failures because it ran `--tests` without checking proptest_vb_vzcuf_PS_004 specifically) |
| **workspace_tests pass count** | **~2718 passed, 44 FAILED** (round-4 was 2912 passed, 39 FAILED — 5 NEW state-11 regressions) |

The slice's strength is that **all 7 round-1+2+3+4 verified CRITICAL/HIGH artifacts remain intact**, AND state-11 produced 5 POSITIVE improvements (the `apply_tail_events` parallel_in_flight fix, 4 new `*_rejects_reserved_seq_sentinel` tests, and 4 new internal `Err(BatchAborted)` assertions). **However, state-11 ALSO introduced 5 NEW CRITICAL test-vs-production contract regressions** by tightening `commit()` on aborted batch to return `Err(BatchAborted)` without updating 5 dependent external tests in `proptest_vb_vzcuf_PS_004.rs` and `journal_side_index_contracts.rs`. The slice cannot advance until these 5 regressions are reconciled (Fix 1 above).

---

## Disposition

| ID | Disposition | Rationale |
|----|-------------|-----------|
| C-R5-1 | `blocker` | 2 proptest failures introduced by state-11's `commit()` contract change. Production code (`batch/types.rs`) is intentionally tighter (SA-002); the proptest fixture must be updated to assert the new contract. CI gate is currently broken. |
| C-R5-2 | `blocker` | 2 workspace_tests proptest failures with same root cause as C-R5-1. The `test_aborted_gate_blocks_subsequent_staging` test is written against the OLD "safe no-op" contract; state-11 replaced it with "aborted batch surfaces typed error". Test must be updated OR production contract rolled back. |
| M-R5-1 | `owner_approved_debt` | Cosmetic: misleading test name `e2e_aborted_batch_commit_succeeds_with_no_persist` contradicts both the assertions and the comment. 1-minute rename. |
| M-R5-2, M-R5-3 | `owner_approved_no_action` | Internal `batch/tests.rs` and `byte_accounting_tests.rs` are gold-standard post-state-11. |
| M-01, M-04, M-06, M-07, M-08, M-09, M-10 | `blocker` | Round-3+4 carry-overs not addressed by state-11. ~15 sites still let mutations pass silently. |
| M-NEW-2 | `blocker` | Round-4 carry-over: 8 smoke tests in `codec/tests.rs` (round-1 H-02 missed). State-11 did not address. |
| L-01, L-02, L-03, L-04, L-05, L-06 | `owner_approved_debt` | Cosmetic + decorative + concrete idioms. |
| O-01 through O-05 | `owner_approved_no_action` | 3 `#[ignore]` tests documented; decorative + concrete idioms documented as gold-standard; benchmark tautology out of behavior-test scope. |

---

## Verdict

```
STATUS: REJECTED
```

**2 NEW CRITICAL + 0 NEW HIGH + 13 NEW/CARRY-OVER MEDIUM + 6 NEW/CARRY-OVER LOW + 5 OBSERVATION findings**, with **0 round-1+2+3+4 regressions** on verified CRITICAL/HIGH artifacts. vb_storage --lib is now GREEN (1552 passed, +1 from derive_dimensions test now passing). **vb_storage --tests has 2 NEW FAILURES** and **workspace_tests has 5 NEW FAILURES** (44 total, was 39 in round 4). State-11 introduced a major contract change (`commit()` on aborted batch returns `Err(BatchAborted)`) but only updated the internal `batch/tests.rs` and `byte_accounting_tests.rs` tests, leaving 5 external tests in `proptest_vb_vzcuf_PS_004.rs` and `journal_side_index_contracts.rs` failing. State-11 ALSO produced 5 POSITIVE improvements: the `apply_tail_events` parallel_in_flight ceiling fix at `tail.rs:63` (un-blocked the `derive_dimensions_includes_action_scheduled_ticket_output` test), 4 new `*_rejects_reserved_seq_sentinel` tests in `keys/tests.rs`, and 4 new internal `Err(BatchAborted)` assertions in `batch/tests.rs`. The slice cannot advance until:

1. **C-R5-1** — update 2 proptest assertions in `proptest_vb_vzcuf_PS_004.rs:43-55,106-116` to expect `Err(BatchAborted)` (~10 min).
2. **C-R5-2** — update 2 proptest assertions in `journal_side_index_contracts.rs:464-498,590-648` to expect the new contract + rename `test_aborted_gate_blocks_subsequent_staging` to reflect new behavior (~20 min).
3. **M-R5-1** — rename `e2e_aborted_batch_commit_succeeds_with_no_persist` in `byte_accounting_tests.rs:786` (~1 min).
4. **M-01, M-04, M-06, M-07, M-08, M-09, M-10, M-NEW-2** — tighten ~25 variant checks across 9 files (~75 min).
5. **C-NEW-1 (carry)** — triage remaining 39 workspace_tests failures from rounds 1-4 (~2-4 hours).

Total cleanup time: ~3-5 hours plus triage of the 39 carry-over workspace_tests failures and the 3 remaining `#[ignore]` tests.

Round-5 progress vs. round 4:
- Round 1: 3 CRITICAL + 10 HIGH + 12 MEDIUM + 8 LOW + 5 OBSERVATION
- Round 2: 2 CRITICAL (new) + 0 HIGH + 6 MEDIUM (new) + 4 LOW (new) + 5 OBSERVATION
- Round 3: 0 CRITICAL + 0 HIGH + 11 MEDIUM + 6 LOW + 3 OBSERVATION
- Round 4: 1 CRITICAL (new — workspace_tests failures) + 0 HIGH + 10 MEDIUM + 6 LOW + 5 OBSERVATION
- Round 5: 2 CRITICAL (new — state-11 contract regressions) + 0 HIGH + 13 MEDIUM + 6 LOW + 5 OBSERVATION

The CRITICAL count increased from 1 to 2 because state-11 introduced 5 new test-vs-production contract regressions that round-4 (which reviewed wave-12 at `bbdd6adec`) could not have detected. The convergence pattern is mixed: state-11 produced 5 POSITIVE improvements (parallel_in_flight fix, 4 new reserved-sentinel tests, 4 new BatchAborted assertions) but also introduced 2 NEW CRITICAL failures by tightening `commit()` contract without updating 5 dependent external tests. The 39 round-4 carry-over workspace_tests failures remain unaddressed.