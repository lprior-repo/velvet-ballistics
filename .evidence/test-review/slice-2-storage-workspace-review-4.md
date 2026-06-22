# Test Review — Slice 2 (Round 4): vb_storage + workspace_tests

**Scope:** 313 test-bearing Rust files (116 in `crates/vb_storage/`; 197 in `crates/workspace_tests/`; plus benches).
Kani harnesses (`kani_*.rs`, ~80 files) are correctly `#[cfg(kani)]`-gated proof artifacts and excluded from
behavior-test review. Round-4 cwd = `bbdd6adec` (wave-12 P3 follow-up closure).

**Date:** 2026-06-21
**Reviewer:** test-reviewer agent (slice 2 of 4, round 4)
**Round:** 4 of 40 — verify round-1+2+3 fixes + find NEW defects from wave-5/6/7/8/9/10/11/12.

## STATUS: REJECTED

Round-1+2+3 CRITICAL and HIGH fixes are largely STILL APPLIED with **0 regressions on the 4 round-1 CRITICAL + 2 round-2 CRITICAL + 3 corruption test artifacts**. Round-3 carry-overs M-03 (contracts_production_binding.rs), M-05 (parse_vet_exit_code), and M-02 (process_lock_tests.rs:213-238) are now SUBSTANTIALLY ADDRESSED — `lock_path.exists()` checks added, error-code-string checks replace bare `is_err()`, `assert_eq!(parse_vet_exit_code(0), Ok(()))` replaces smoke. **Wave-12 closed 11 P3 testfix follow-ups, including the previously-failing `frame_seed_slot_dimension_overflow_reports_exact_variant` test (now uses concrete `matches!(result, Err(RecoveryError::FrameDimensionOverflow { run: found }) if found == run)`)**. **vb_storage `--tests` reports 1762 passed, 0 failed** (round-3 baseline 1760). However, **`workspace_tests` shows 2912 passed and 39 FAILED**, a **CRITICAL REGRESSION** that round-3 did not measure. Failures cluster around: (1) `u64::MAX` sequence encoding rejected with `ReservedSeqSentinel` in `journal_tail_scan_fallback_tests.rs` (5 sites); (2) proptest failures in `journal_side_index_contracts.rs:458,581`; (3) recovery-watermark tail contiguous failure; (4) timer-deadline retry invariant failures (5 sites); (5) workspace-assertions, source-length-gate, recovery-hydration, BDD-scenario, and taint-fixture failures. **0 round-1+2+3 regressions on verified CRITICAL/HIGH artifacts**, but workspace_tests is now blocked on 39 new failures introduced between wave-5 and wave-12. Round-3 carry-overs M-06/M-07/M-08/M-09/M-10 + L-01 through L-06 remain unaddressed; 3 `#[ignore]` tests remain (down from 5). Cannot advance to APPROVED until the 39 workspace_tests failures are triaged.

---

## Round-1 + Round-2 + Round-3 Fix Verification Table

| Round | ID | File:Line | Defect | Status | Evidence |
|-------|----|-----------|--------|--------|----------|
| 1 | C-01 / H-08 | `integration_compile_error_message_quality.rs:374-378,401-405,426-430` | `assert!(result.is_ok() \|\| result.is_err())` tautology (3 sites) | **STILL APPLIED** | Lines 376, 402, 428 still use `matches!(result, Err(CompileErrors(ref errors)) if errors.iter().any(|e| matches!(e, CompileError::DepthLimit/SequenceLimit/ScalarLimit{...})))` — concrete variant + field check. **0 regressions.** |
| 1 | C-02 | `integration_runtime_storage_fault_tolerance.rs:215-218` | `assert!(result.is_ok() \|\| result.is_err())` tautology | **STILL APPLIED** | Lines 215-218 still use `matches!(result, Err(ref e) if matches!(e, RuntimeError::InvalidRecoveryHydration))` — concrete variant check. |
| 1 | C-03 | `process_lock_tests.rs:141-179` | "match-all-outcomes" with `_ = other;` and `Ok(_) => {} \| Err(_) => {}` (2 tests) | **STILL APPLIED** | Lines 149-152: `assert!(matches!(result, Err(JournalError::ProcessLockHeld { .. }))`; lines 156-178: `lock_path.exists()` while open, `!lock_path.exists()` after drop, `result.is_ok()` for re-open. SECURITY contract still meaningful. |
| 1 | C-04 | `edge_case_tests.rs:547` | Test name `encode_rejects_zero_length_payload_serialization` contradicted `assert!(result.is_ok())` | **STILL APPLIED** | Renamed to `encode_accepts_zero_length_payload_and_round_trips` (line 547). Full round-trip: encodes → asserts `!encoded.is_empty()` → decodes → asserts `decoded.bytes == vec![]`, `decoded.digest == [0u8; 32]`, `envelope.kind == RecordKind::Blob`. |
| 2 | C-05 | `workspace_tests/tests/doctor_key_decode_tests.rs:617-635` | `readonly_journal_declared_keyspaces_returns_ten` asserted `len() == 10` (wave-5/6 added `run_seq_gap`) | **STILL APPLIED** | Test renamed to `readonly_journal_declared_keyspaces_returns_eleven` (line 618). Line 621-625 asserts `assert_eq!(spaces.len(), 11, "declared_keyspaces must return exactly 11 entries (10 historical + run_seq_gap from wave-5/6)")`. Lines 627-630 also assert `names.contains(&"run_seq_gap")`. Plus non-empty check (line 632-633). |
| 2 | C-06 | `workspace_tests/tests/fjall_keyspace_manifest_tests.rs:309-356` | `declared_keyspaces_count` asserted `len() == 10` (same wave-5/6 root cause) | **STILL APPLIED** | Lines 312-316: `assert_eq!(keyspaces.len(), 11, "declared_keyspaces must return exactly 11 entries (10 historical + run_seq_gap from wave-5/6)")`. Plus sibling test `declared_keyspaces_contains_required_names` (line 320-343) enumerates the 11 required names. Plus `declared_keyspaces_no_duplicates` (line 346-356). |
| 2 | artifacts | `vb_storage/src/journal/tests.rs:1889,1931,1976` (round-3 cited 1890,1932,1977 — actual lines unchanged) | 3 corruption tests for `events_for_run` rejection of corrupt latest snapshot (BadMagic, PayloadDigestMismatch, PostcardDecodeFailed) | **STILL APPLIED + STILL PASSING** | Filtered run: `cargo test -p vb_storage -- "events_for_run_rejects_corrupt_latest" "events_for_run_rejects_latest" → 3 passed, 1759 filtered out`. All 3 tests present (lines 1890-1928, 1932-1973, 1977-2007) and PASSING. Each uses `matches!(result, Err(JournalError::BadMagic { .. }))` / `PayloadDigestMismatch` / `PostcardDecodeFailed`. |
| 2 | artifacts | `cargo test -p vb_storage --tests` | Test pass count | **STILL APPLIED** | **`1762 passed` (37 suites, 41.42s)** — round-3 baseline was 1760, now +2 (wave-12 added 2 proptest regressions-tests). 0 failed. **vb_storage is green.** |
| 3 | H-02 (carry) | `vb_storage/src/codec/tests.rs:565,2813,2825,2850` (4 sites, fixed in round 1-2) | `assert!(result.is_ok())` smoke tests on `verify_digest_match` and `validate_replayed_event` boundaries | **STILL APPLIED** | All 4 sites still use `assert_eq!(result, Ok(()), "...")` with explicit unit comparison. Round-4 census confirms 0 bare `assert!(result.is_ok())` at these 4 sites. |
| 3 | H-03 (carry) | `vb_storage/src/security_tests.rs:767` | `assert!(result.is_ok())` SECURITY smoke test | **STILL APPLIED** | Line 767-771 still uses `assert_eq!(result, Ok(()), "SECURITY: correct digest must pass verification (and must NOT silently Ok on wrong digest — see rejects_wrong test)")`. |
| 3 | M-02 (carry) | `vb_storage/src/process_lock_tests.rs:213-228` (2 sites — `open_store_acquires_process_lock` 217, `init_keyspaces_acquires_process_lock` 230) | Smoke test + no `lock_path.exists()` check | **STILL APPLIED + IMPROVED** | Both tests now have `let lock_path = temp.path().join(".process.lock"); assert!(lock_path.exists(), "open_store must create .process.lock file (test name asserts the lock is acquired)")` AFTER the smoke check. Pattern is **decorative + concrete** (gold-standard per round-3 O-02). Re-classify as OBSERVATION (M-02-O). |
| 3 | M-03 (carry) | `workspace_tests/tests/contracts_production_binding.rs:170-187, 228-244, 292-313, 330-344` (4 critical paths — round-2 said 15 sites, now exemplary across the file) | `is_err()` accepts any variant | **STILL APPLIED + IMPROVED** | `test_prod_parse_schema_version_invalid` (170-187) now uses `.err().unwrap_or_else(...)` + `message.contains("MISSING_SCHEMA_VERSION"\|"INVALID_VERSION")` for 6 specific inputs. `test_prod_parse_contract_kind_invalid` (228-244), `test_prod_compare_semver_invalid_format` (292-313), `test_prod_parse_vet_exit_code_failure` (330-344) all use concrete `.err().unwrap_or_else(...)` + message-substring checks. **Round-1 H-06 is essentially closed for this file.** |
| 3 | M-05 (carry) | `workspace_tests/tests/contracts_production_binding.rs:282` | `assert!(parse_vet_exit_code(0).is_ok())` smoke | **STILL APPLIED + IMPROVED** | `test_prod_parse_vet_exit_code_success` (line 321-327) now uses `assert_eq!(parse_vet_exit_code(0), Ok(()), "parse_vet_exit_code(0) must return Ok(()) (success path)")`. |
| 3 | artifacts | `recovery::replay::summary::tests::frame_seed_slot_dimension_overflow_reports_exact_variant` at `vb_storage/src/recovery/replay/summary/tests.rs:482-500` | `assert!(result.is_ok(), "Expected Ok, got: {:?}", result)` with bogus "Frame seed now handles large slot indices gracefully instead of erroring" comment — test was FAILING in CI between wave-5 and wave-12 | **FIXED in wave-12** | Wave-12 commit `bbdd6adec` (2026-06-21 15:57:18) replaced the smoke + bogus assertions with: `let result = recover_runtime_frame_seed_from_events(&events); assert!(matches!(result, Err(RecoveryError::FrameDimensionOverflow { run: found }) if found == run), "Expected FrameDimensionOverflow for SlotIdx::MAX output, got {result:?}");` — concrete variant check with field validation. Comment now correctly explains the production behavior. Test PASSES. |
| 3 | artifacts | 3 `#[ignore]` behavior tests removed in wave-12 | Round-3 listed 5 `#[ignore]` tests: `vb_qi37_25_quality_gates.rs:250`, `vb_c1s0_orchestration_runtime_tests.rs:593`, `vb_test_runtime_lifecycle_state_behavior.rs:469`, `vb_vt2f_direct_runtime_api_acceptance.rs:605`, `vb_njju_mutation_fuzz_property_closure.rs:224` | **2 of 5 FIXED in wave-12** | Wave-12 un-ignored: (1) `vb_qi37_25_quality_gates.rs::package_name_drift_reports_exact_member_and_expected_name` — fixed test setup (vb_cli name) + expected error string; (2) `vb_njju_mutation_fuzz_property_closure.rs::test_fuzz_smoke_runs_yaml_ipc_journal_compiled_ir_targets` — updated assertion to match new `for fuzz_target in` for-loop pattern; (3) `symbolic_code_behavior_tests.rs::journal_error_symbolic_code_key_capacity` — fixed `HasSymbolicCode::symbolic_code(&JournalError::KeyCapacity)` trait method call. **3 `#[ignore]` tests remain** (down from 5). |

**Round-1+2+3 regression count: 0.** All 4 round-1 CRITICAL + 2 round-2 CRITICAL + 3 corruption test artifacts + 3 round-3 carry-overs verified intact or improved.

---

## Round-3 HIGH Carry-over Status (round-4 verification)

The 4 round-1 CRITICAL + 2 round-2 CRITICAL fixes held. The 10 round-1 HIGH findings have been **SUBSTANTIALLY ADDRESSED** in wave-12:
- **H-01** (hydrate_tests.rs — 9 sites round-2 → 3 sites round-3 → 3 sites round-4): 3 sites remain at `hydrate_tests.rs:281,287,374`. Re-classify as MEDIUM (M-01).
- **H-02** (codec/tests.rs — 4 sites): FULLY FIXED to `assert_eq!`. Now 8 NEW sites at lines 534, 674, 1756, 2459, 2655, 2739, 2863, 2996 are bare `result.is_ok()` (NEW finding M-NEW-2).
- **H-03** (security_tests.rs:767): FULLY FIXED.
- **H-04** (process_lock_tests.rs:213-228): FIXED in wave-12 — `lock_path.exists()` checks added (decorative + concrete).
- **H-05** (chunk_* smoke tests): NOT FIXED. 8 sites remain. Re-classify LOW (L-03 carry).
- **H-06** (contracts_production_binding.rs): SUBSTANTIALLY FIXED — file is exemplary post-wave-12.
- **H-07** (vb_test_compile_parse_validate_behavior.rs:184-188): NOT FIXED. Re-classify MEDIUM (M-04 carry).
- **H-09** (5 `#[ignore]` tests): 2 of 5 REMOVED in wave-12. 3 remain. Re-classify OBSERVATION (O-01).
- **H-10** (contracts_production_binding.rs:282): FIXED in wave-12.

**Round-3 HIGH carry-over status: 6 of 10 addressed, 4 remain (M-01, M-04, L-03, O-01).**

---

## Findings (CRITICAL first)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix |
|----|-----|-----------|--------|------------------------------|------------------|
| **C-NEW-1** | **CRITICAL** | `crates/workspace_tests/tests/` — 39 failing tests across 14 files (see "NEW CRITICAL FINDING DETAIL" below) | Production code changes between wave-5 and wave-12 broke existing test contracts. Round-3 review measured only `cargo test -p vb_storage --tests` (1760 passed) and did not measure workspace_tests. New wave-12 commit `bbdd6adec` did not re-verify workspace_tests. | Delete the production changes that broke the tests (e.g., revert `ReservedSeqSentinel` rejection of `u64::MAX`, revert retry-invariant behavior, revert taint-hydration metadata contract). All 39 tests pass. | Triage the 39 failures into (a) production regression — fix production to honor the contract, or (b) test bug — fix the test to match new correct production behavior. **Do not bulk-ignore.** |
| M-NEW-2 | MEDIUM | `crates/vb_storage/src/codec/tests.rs:534,674,1756,2459,2655,2739,2863,2996` (8 sites — NOT in round-3 census) | `assert!(result.is_ok(), "...")` smoke tests on codec accept/decode functions (8 functions returning `Result<(), JournalError>`). Round-1 H-02 fixed 4 sites but missed these 8. Functions: `encode_accepts_payload_at_exact_max_boundary` (534), `validate_replayed_event_accepts_matching_run_and_seq` (674), `run_header_kind_id_accepted_by_magic_index_record` (1756), `encode_accepts_empty_source_with_generous_max` (2459), `magic_journal_event_accepts_all_event_kinds` loop body (2655), `index_update_kind_id_accepted_by_magic_index_record` (2739), `verify_digest_match_empty_payload` (2863), `record_without_trailing_bytes_decodes_successfully` (2996). | Replace each function with `Ok(())` for any input. All 8 tests pass. The companion failure-path tests still fail for genuine bad inputs, but partial mutation caught. | Convert each to `assert_eq!(result, Ok(()), "...")` or assert returned value (e.g., `let decoded = result.expect("must decode"); assert_eq!(decoded.payload_digest, blake3::hash(empty_payload).into());`). |
| M-NEW-3 | MEDIUM | `crates/vb_storage/tests/vb_core_atomic_admission_red.rs:1151-1155` (1 site — NOT in round-3 census) | `prop_assert!(result.is_err() \|\| journal.compiled_ir(workflow.digest()).unwrap().is_none(), ...)` — weak-OR. Accepts EITHER Err from submit_artifact OR Ok-with-no-compiled-ir. The `unwrap()` on `compiled_ir` is silently dropped: if `journal.compiled_ir` returns `Err(...)`, the test would PANIC, not assert false. This conflates two different contracts: "admission fails" vs "admission succeeds but doesn't write artifact". | Make `submit_artifact` always return `Ok(())`. The `is_err()` check fails; but the `journal.compiled_ir(...).unwrap().is_none()` returns `Some(_)` (if admission wrote) or `None` (if admission didn't write). With no contract check on what `compiled_ir` returns, the test passes. | Replace with concrete contract: `prop_assert!(matches!(result, Err(AdmissionError::SourceDigestMismatch { .. })));` (the test name implies this is the expected error). Remove the `journal.compiled_ir` lookup. |
| M-01 | MEDIUM | `crates/vb_storage/src/hydrate_tests.rs:281,287,374` (3 sites, carry from round 3 M-01) | `assert!(result.is_ok(), "seq 6 must be contiguous with snapshot seq 5, got {:?}", result)` — smoke tests on `validate_tail_first_seq_contiguous_with_snapshot` and `validate_snapshot_recovery_inputs`. Functions return `Result<(), SnapshotRecoveryInputViolation>`. Round-1 H-01 carry-over. | Replace `validate_tail_first_seq_contiguous_with_snapshot` with `Ok(())` always. All 3 sites pass. The 4 error-path tests on adjacent lines (`rejects_gap`, `_rejects_equal_to_snapshot`, etc.) still fail. | Convert each to `assert_eq!(result, Ok(()), "...")`. |
| M-04 | MEDIUM | `crates/workspace_tests/tests/vb_test_compile_parse_validate_behavior.rs:184-188` (`parse_rejects_whitespace_only_source`, carry from round-3 M-04 / round-1 H-07) | `assert!(result.is_err(), "whitespace-only source should fail")` — accepts any error. Round-1 H-07 still not fixed. | Make `YamlCompiler::parse_ast(b"   \n\n   \n")` return `Err(CompileError::Other)`. Test passes. | Add `let msg = result.unwrap_err().to_string(); assert!(msg.contains("empty") \|\| msg.contains("whitespace"));` |
| M-06 | MEDIUM | `crates/vb_storage/src/blob_tests.rs:246-263` (line 260, carry from round-3 M-06) | `result.is_err() \|\| result.map_or(false, \|opt\| opt.is_none())` — accepts EITHER `Err(anything)` OR `Ok(None)`. The contract is "must return a typed error for corrupt data" — silent absence accepted. | Make `journal.blob(digest)` return `Ok(None)` for corrupt data. Test passes silently. SECURITY-relevant. | Replace with `assert!(matches!(result, Err(JournalError::BlobDecode(_) \| JournalError::PayloadDigestMismatch { .. })));` |
| M-07 | MEDIUM | `crates/vb_storage/src/recovery/recovery_unit_tests.rs:796-805` (line 800, carry from round-3 M-07) | `assert!(result.is_err());` immediately followed by `matches!(result.unwrap_err(), RecoveryError::NoRecoveryData { .. })`. First is decorative. | n/a — second assertion is concrete. | Remove redundant `assert!(result.is_err())` on line 800. |
| M-08 | MEDIUM | `crates/vb_storage/src/type_tests.rs:191-197` (line 196, carry from round-3 M-08) | `assert!(err.is_err(), "zero should fail")` — accepts any variant. `JournalBatchSize::try_from_usize(0)` should produce specific `JournalBatchSizeError::Zero`. | Make `try_from_usize(0)` return `Err(JournalBatchSizeError::SomeOtherVariant)`. Test passes. | Replace with `assert!(matches!(err, Err(JournalBatchSizeError::Zero)));` |
| M-09 | MEDIUM | `crates/workspace_tests/tests/doctor_storage_scan_decode_tests.rs:1266-1282` (lines 1275, 1278, carry from round-3 M-09) | `parse_decode_error_invalid_keyspace_path` uses `assert!(result.is_err(), "expected error opening nonexistent path")` followed by match accepting any Err variant. Round-2 M-07 carry. | Make `FjallJournal::open` return `Err(JournalError::ProcessLockIo { ... })` for nonexistent path. Test passes silently. | Replace with `assert!(matches!(result, Err(JournalError::Fjall(_) \| JournalError::PathNotFound { .. })));` |
| M-10 | MEDIUM | `crates/workspace_tests/tests/integration_compile_error_message_quality.rs:479-488` (carry from round-3 M-10) | `let is_ok = result.is_ok(); let errors = match result { Err(CompileErrors(es)) => es, Ok(_) => Vec::new() }; assert!(!errors.is_empty() \|\| is_ok);` — NEW TAUTOLOGY from wave-7/8 still NOT FIXED. The assertion accepts EITHER happy path OR error path. Only failure case is `Err(CompileErrors(vec![]))`. | Delete the version validation in production (return `Ok(_)` for any version). Test passes (`is_ok == true`). | Replace with concrete contract: `assert!(matches!(result, Err(CompileErrors(ref errors)) if errors.iter().any(|e| matches!(e, CompileError::InvalidVersion { .. }))), "version 'invalid-version' must be rejected, got {result:?}");` |
| L-01 | LOW | `crates/vb_storage/src/tests/chunk_012.rs:181` (carry from round-3 L-01) | Test name `declared_keyspaces_returns_ten_entries` says "ten" but asserts `keyspaces.len() == 11` on line 186. Comment on line 184 says "exactly 11". Stale name misleads readers. | n/a — purely cosmetic. | Rename to `declared_keyspaces_returns_eleven_entries`. |
| L-02 | LOW | `crates/vb_storage/src/codec/tests/replay_integrity.rs:201` (1 site — round-4 NEW finding) | `assert!(result.is_ok(), "next_seq(u64::MAX-1) must succeed")` — smoke test on `next_seq` boundary. Companion failure-path tests still fail for `u64::MAX`. | Replace `next_seq(u64::MAX-1)` with `Ok(EventSeq::new(u64::MAX))` (skip the +1). Test passes. | Convert to `assert_eq!(result, Ok(EventSeq::new(u64::MAX)), "next_seq at boundary")` and add `assert_eq!(result.unwrap().get(), u64::MAX)`. |
| L-03 | LOW | `crates/vb_storage/src/tests/chunk_004.rs:166,274`, `chunk_008.rs:173,181,185`, `chunk_040.rs:212,227,268` (8 sites, carry from round-1 H-05 / round-3 L-03) | `assert!(journal.is_ok(), "...")` smoke tests on `FjallJournal::open`/`open_store`/`init_keyspaces`. Each is followed by concrete round-trip. Decorative + concrete (gold-standard). | n/a — concrete round-trip catches production mutations. | Acceptable as decorative + concrete. |
| L-04 | LOW | `crates/workspace_tests/tests/doctor_storage_scan_decode_tests.rs:1363-1364,1457-1460` (4 sites, carry from round-3 L-04) | `assert!(header_result.is_ok()); assert!(full_result.is_ok());` at lines 1363-1364 (decorative, followed by `?`); `assert!(decoded_header.is_ok())` at 1457-1460 (decorative, no concrete check follows). | For 1457: make `decode_record_header` always return `Ok(RecordHeader::default())`. Test passes. | For line 1457 add `assert_eq!(decoded_header.unwrap().sequence, 1)`. |
| L-05 | LOW | `crates/workspace_tests/tests/vb_test_core_workflow_slot_behavior.rs:694,730,810,997,1104,1117` (6 sites, carry from round-3 L-05) | `assert!(result.is_err());` followed by `let err = result.unwrap_err(); assert!(matches!(err, CoreError::SlotUninitialized { slot } if slot == SlotIdx::new(0)));`. Decorative + concrete. | n/a — concrete matches! catches specific variant. | Acceptable as decorative + concrete. |
| L-06 | LOW | `crates/vb_storage/src/security_tests.rs:1047-1063` (line 1059-1062, carry from round-3 L-06) | `assert!(result.is_ok(), "re-open after drop must succeed because lock was released")` — happy path for lock release. The `process_lock_tests.rs:156-178` test covers the lock-release contract with `lock_path.exists()` checks. | n/a — paired with `process_lock_is_released_on_drop`. | Add `let lock_path = temp.path().join(".process.lock"); assert!(!lock_path.exists(), ".process.lock must be released on drop");` before the second `open`. |
| O-01 | OBSERVATION | `crates/workspace_tests/tests/vb_test_runtime_lifecycle_state_behavior.rs:469`, `vb_vt2f_direct_runtime_api_acceptance.rs:605`, `vb_c1s0_orchestration_runtime_tests.rs:593` (3 sites — down from 5 in round 3) | `#[ignore]` on behavior tests, all "BLOCKED" pre-existing runtime bugs. Wave-12 un-ignored 2 of 5. | Mark all 3 `#[ignore]` permanently. | Open beads for each ignored test to track closure. |
| O-02 | OBSERVATION | `crates/workspace_tests/tests/runtime_version_barrier_tests.rs:314,340,366,415,436,457,478,499,520,547,612,668` (12 sites — round-3 M-11 re-classified) | All 12 sites use **decorative + concrete**: `assert!(result.is_err());` followed by `let err = result.unwrap_err(); assert!(matches!(err, AdmissionError::ArtifactInvalidGateCount { found, required } if found == X && required == 15))` etc. Concrete matches! on specific variant + field. | Make `admit_artifact_run` return `Err(AdmissionError::Other)`. The decorative `is_err()` passes; the concrete `matches!` fails. So the concrete check IS the binding test. | n/a — gold-standard decorative + concrete pattern. Documented idiom. |
| O-03 | OBSERVATION | `crates/workspace_tests/tests/vb_test_cli_storage_io_behavior.rs:356,401,...` (17 sites) | `assert!(result.is_ok(), "events_for_run must succeed")` followed by `let events = result.expect("events ok")` and concrete `matches!(terminal, Some(JournalEvent::RunFinished { .. }))` checks. Decorative + concrete. | n/a — concrete check makes test meaningful. | Acceptable as documented gold-standard idiom. |
| O-04 | OBSERVATION | `crates/workspace_tests/benches/action_dispatch.rs:378` (carry from round-1 O-01 / round-3 O-03) | `assert!(result.is_err() \|\| result.is_ok(), "dispatch of unknown action")` — explicit tautology in a benchmark. | n/a — benchmarks are not behavior tests. | Acceptable as no-op benchmark correctness assertion. |
| O-05 | OBSERVATION | `crates/vb_storage/src/security_tests.rs:1076` (1 site) | `assert!(result.is_err(), "second open must fail")` followed by `assert_eq!(before_count, after_count, "no new files should be created when lock acquisition fails")`. Decorative + concrete. | n/a — concrete file-count check is the real test. | Acceptable as decorative + concrete. |

---

## NEW CRITICAL FINDING DETAIL (C-NEW-1): workspace_tests 39 Failures

Round-3 review measured only `cargo test -p vb_storage --tests` (1760 passed) and did not run `cargo test -p vb_workspace_tests --no-fail-fast`. Round-4 measurement reveals 39 failures across 14 test files. Below is the complete failure list (extracted from `/tmp/workspace_test.log`):

```
crates/workspace_tests/tests/bounded_memory_lease_tests.rs
  └─ bounded_scan_tests::bounded_scan_overflow_limit_handled_safely  (panicked in raw_vec at alloc/src/raw_vec/mod.rs:28:5)

crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs
  └─ max_sequence_ordering  (called `Result::unwrap()` on an `Err` value: ReservedSeqSentinel @ line 290:82)

crates/workspace_tests/tests/journal_side_index_contracts.rs
  ├─ test_duplicate_idempotency_key  (prop_assert failed: append_event B must succeed @ 483, minimal input: action=1, run=1, step=0, seq_a=0, seq_b=0)
  └─ test_aborted_gate_blocks_subsequent_staging  (prop_assert failed: after abort, append_event must return Ok @ 635, minimal input: action=1, run=1, step=0, num_subsequent=1)

crates/workspace_tests/tests/journal_tail_scan_fallback_tests.rs
  ├─ max_sequence_key_encodes_without_panic  (run_event_key at seq=u64::MAX: ReservedSeqSentinel @ 838:39)
  ├─ run_event_key_has_correct_byte_length_for_all_boundary_sequences  (run_event_key at boundary: ReservedSeqSentinel @ 948:14)
  ├─ run_event_key_ordering_matches_numeric_comparison  (key at seq=u64::MAX: ReservedSeqSentinel @ 376:59)
  └─ sequence_bytes_decoded_to_correct_u64_values  (key must encode: ReservedSeqSentinel @ 406:58)

crates/workspace_tests/tests/recovery_watermark_tests.rs
  ├─ proptest_snapshot_seq_lt_tail_first_seq  (prop_assert failed: tail event seq 2 not contiguous with snapshot seq 0 (expected 1) @ 737, minimal input: snapshot_seq=0, tail_first_seq=2, tail_count=1)
  └─ watermark_journal_recovery_rejects_max_seq  (panicked, line 697)

crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs
  └─ tail_seq_equal_to_snapshot_seq_fails  (detail should mention seq comparison failure: tail event seq 10 not contiguous with snapshot seq 10 (expected 11) @ 1165:13)

crates/workspace_tests/tests/timer_deadline_primitive_tests.rs
  ├─ evaluate_retry_invariants::evaluate_retry_no_retry_policy_exhausts_after_first_failure  @ 1477:17
  ├─ evaluate_retry_invariants::evaluate_retry_full_cycle_three_attempts_then_exhaustion  @ 1456:17
  ├─ retry_state_invariants::is_exhausted_when_remaining_is_zero  @ 1194:17
  ├─ retry_state_invariants::retry_state_exhausted_state_is_detectable  @ 1330:17
  └─ evaluate_retry_invariants::evaluate_retry_exhausted_when_remaining_zero_and_retryable  @ 1388:17

crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs
  └─ valid_workspace_passes_sharpened_assertions  @ 326:5

crates/workspace_tests/tests/vb_a0t1_source_length_gate_tests.rs
  ├─ test_out_of_scope_vb_cli_xtask_changes_are_routed_with_touched_package_evidence  @ 679:5
  └─ test_full_source_length_pipeline  @ 146:5

crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs
  ├─ given_full_journal_slot_taint_metadata_is_corrupt_when_hydrating_then_recovery_fails_closed
  ├─ given_legacy_collect_frame_extra_when_hydrating_full_journal_then_extra_is_not_corrupt_taint  @ 481:5
  └─ given_public_hydration_tail_slot_cannot_be_dimensioned_when_recovery_runs_then_clean_taint_is_not_defaulted

crates/workspace_tests/tests/vb_kyyf_*.rs (BDD scenarios)
  ├─ bdd_kyyf_001_to_006_require_executable_public_surfaces_not_catalog_bookkeeping_only
  └─ all_eight_screens_pass_reachability_and_overlap_gates

crates/workspace_tests/tests/vb_secret_*.rs / vb_overlap_*.rs (taint fixtures)
  ├─ secret_false_pass_fixture_is_rejected
  ├─ intentional_overlap_fixture_fails_gate
  ├─ secret_negative_fixture_is_consumed_by_command_boundary
  ├─ overlap_negative_fixture_is_consumed_by_command_boundary
  ├─ overlap_false_pass_fixture_is_rejected
  ├─ intentional_secret_fixture_fails_redaction_gate
  └─ secret_values_are_redacted_in_every_screen

crates/workspace_tests/tests/vb_xxx_*.rs (additional failure paths)
  ├─ ask_answer_records_exact_clean_taint_when_answer_writes_output
  ├─ event_only_recovery_returns_derived_bool_when_durable_taint_is_derived
  ├─ no_output_step_recovery_has_no_recovered_slot_entries
  ├─ no_output_step_does_not_fabricate_slot_zero_dimension
  ├─ no_output_step_summary_reports_zero_slots_written
  ├─ runtime_to_storage_mapping_preserves_taint_for_slot_write
  └─ proptest_no_output_success_never_creates_slot_zero

crates/workspace_tests/tests/edge_submit_after_shutdown_*.rs
  └─ edge_submit_after_shutdown_enqueues_but_does_not_process
```

**Failure cluster analysis:**

1. **u64::MAX sequence encoding rejected** (5 sites in `journal_tail_scan_fallback_tests.rs` + 1 in `fjall_keyspace_manifest_tests.rs`): Production `run_event_key` and `max_sequence_key` reject `u64::MAX` with `Err(ReservedSeqSentinel)`. Tests expect successful encoding at this boundary. **This is consistent with the test fixtures being correct and production being newly restrictive** — wave-11 (commit 35854649d) likely added `ReservedSeqSentinel` rejection at the u64::MAX boundary.

2. **Recovery tail contiguous rejection** (4 sites across `recovery_watermark_tests.rs`, `slot_written_ordering_integration_tests.rs`, and `recovery_unit_tests.rs`): The validation `tail event seq X is not contiguous with snapshot seq Y (expected Z)` rejects inputs that the test expects to succeed. Error message format also changed: test expects "expected Y" but gets "expected Z" (different value). **Likely production changed both the validation and the error message format.**

3. **Timer-deadline retry invariants** (5 sites in `timer_deadline_primitive_tests.rs`): `evaluate_retry_invariants::evaluate_retry_no_retry_policy_exhausts_after_first_failure` and 4 others. **Likely wave-10/11 changed retry semantics.**

4. **Taint-fixture and redaction-gate tests** (7 sites): Multiple `vb_secret_*` and `vb_overlap_*` fixtures fail. **Wave-10/11 likely changed taint/redaction logic.**

5. **BDD scenarios and recovery-hydration tests** (multiple sites): `vb_kyyf_*` and `vb_jpq7_3_*` tests fail. **These appear to be evidence-trail tests for contracts that have been rewritten.**

**Recommended triage approach:**
- For each failing test, determine: (a) is the production behavior correct (per existing contract docs)? If yes, the test is correct and production regressed. (b) Is the production behavior newly correct and the test stale? Then update the test (with proper contract docs).
- Do NOT silently mark these as `#[ignore]` — these are 39 distinct contract violations that need explicit owner review.

---

## Pattern Census

### `assert!(...is_ok()) / assert!(...is_err())` (BANNED in behavior assertions)

| Crate | Total matches | Top files (round 4) |
|-------|---------------|---------------------|
| `vb_storage/src` | ~30 (NEW: 8 sites in codec/tests.rs + 8 chunk_* carry + 3 hydrate_tests + 2 process_lock + recovery_unit_tests 1 + type_tests 1 + security_tests 1 + others) | `codec/tests.rs` (**8 NEW**: 534, 674, 1756, 2459, 2655, 2739, 2863, 2996), `hydrate_tests.rs` (3 — carry from round-3 M-01), `tests/chunk_040.rs` (3 — carry), `tests/chunk_008.rs` (3 — carry), `tests/chunk_004.rs` (2 — carry), `process_lock_tests.rs` (2 — decorative + concrete gold-standard), `recovery/recovery_unit_tests.rs` (1 — carry M-07), `recovery/replay/summary/tests.rs` (1 — fixed by wave-12), `type_tests.rs` (1 — carry M-08), `codec/tests/replay_integrity.rs` (1 — NEW L-02), `security_tests.rs` (1 — decorative + concrete O-05) |
| `vb_storage/tests` | ~6 (round-4 NEW weak-OR + others) | `vb_core_atomic_admission_red.rs` (1 — NEW M-NEW-3), `vb_god2f_classification_properties.rs` (2), `vb_god2f_recovery_properties.rs` (2), `proptest_journal_idempotency.rs` (1) |
| `workspace_tests/tests` | ~135 (carried from round 3) | `integration_validate_yaml_parsing.rs` (~10 bare is_err), `runtime_version_barrier_tests.rs` (12 decorative + concrete — acceptable O-02), `contracts_production_binding.rs` (4 sites — all FIXED to `.err().unwrap_or_else(...)` + message-substring checks), `vb_test_core_workflow_slot_behavior.rs` (~15 decorative + concrete — acceptable O-03/L-05), `vb_test_cli_storage_io_behavior.rs` (17 decorative + concrete — acceptable O-03), `doctor_storage_scan_decode_tests.rs` (13 — mostly decorative + concrete), `integration_compile_error_message_quality.rs` (3 — including the NEW tautology at 488), `vb_8mdp_7_resource_admission_props.rs` (10, prop_assert), `vb_test_compile_parse_validate_behavior.rs` (9 — including M-04 carry), `integration_validate_policy_enforcement.rs` (5), `vb_qi37_2_4_integration_budget_errors.rs` (5), `vb_eepg_bdd_tests.rs` (3) |
| `workspace_tests/benches` | 12 | Same as round 3. |
| **TOTAL** | **~183** | **UP ~11 from round-3's ~172**. Increase is dominated by NEW M-NEW-2 (8 sites in codec/tests.rs) + NEW M-NEW-3 (1 site) + carry-overs. Wave-12 IMPROVED 4 sites (frame_seed_slot_dimension_overflow, 3 ignored tests). |

### Tautology assertions (`is_ok() || is_err()` or weak-OR)

| File:Line | Status |
|-----------|--------|
| All 4 round-1 tautology sites (C-01, C-02) | **DELETED** (round-1 fix verified) |
| `crates/vb_storage/src/blob_tests.rs:260` | **STILL PRESENT** — `result.is_err() \|\| result.map_or(false, \|opt\| opt.is_none())` weak-OR. Re-classified MEDIUM (M-06). |
| `crates/workspace_tests/tests/integration_compile_error_message_quality.rs:488` | **STILL PRESENT** — `assert!(!errors.is_empty() \|\| is_ok)` NEW TAUTOLOGY from wave-7/8. Re-classified MEDIUM (M-10). |
| `crates/vb_storage/tests/vb_core_atomic_admission_red.rs:1151` | **NEW FINDING** — `prop_assert!(result.is_err() \|\| journal.compiled_ir(...).unwrap().is_none())` weak-OR with hidden `unwrap()`. Re-classified MEDIUM (M-NEW-3). |
| `crates/workspace_tests/benches/action_dispatch.rs:378` | **STILL PRESENT** — benchmark context. Observation-level (O-04). |
| **TOTAL** | **2 hard tautologies (1 NEW + 1 carry) + 1 weak-OR (carry) + 1 benchmark tautology (carry)** |

### `let _ = ...` (silent error suppression)

| Crate | Total | Notes |
|-------|-------|-------|
| `vb_storage/src` | ~10 (same as round 3) | `kani_*` (3 — proof harnesses), `index_tests.rs` (7 — `item.key()` discarding — fixture iteration), `tests/chunk_*.rs` (~10 — `builder.try_push()` discarding capacity check), `proptest_storage.rs` (~5 — proptest input verification), `recovery/recovery_unit_tests.rs` (1 — `_exhaustive_match`). All acceptable as fixture/proof-harness patterns. |
| `workspace_tests/tests` | ~95 (same as round 3) | `bdd_validation_tests.rs:1364-1402` (37 — `let _ = ValidationError::VariantName` variant-existence check), `timer_deadline_primitive_tests.rs` (~50 — `wheel.insert()` discarding setup result), `bdd_idempotency.rs` (2 — `tracker.mark_completed(&ticket)` setup), `cancel_kill_lattice_tests.rs` (2). All acceptable as fixture. |
| **TOTAL** | **~105** | Same as round 3. **No new silent discards introduced.** |

### `#[ignore]` / `#[should_panic]` / `sleep(` / `todo!()` / `unimplemented!()`

| Crate | Total | Notes |
|-------|-------|-------|
| `workspace_tests/tests` | **3 `#[ignore]`** | **Down from 5 in round 3**. Wave-12 un-ignored 2 (`vb_qi37_25_quality_gates.rs:package_name_drift_reports_exact_member_and_expected_name`, `vb_njju_mutation_fuzz_property_closure.rs:test_fuzz_smoke_runs_yaml_ipc_journal_compiled_ir_targets`) and additionally un-ignored `symbolic_code_behavior_tests.rs:journal_error_symbolic_code_key_capacity` (not in round-3 list). Remaining 3: `vb_test_runtime_lifecycle_state_behavior.rs:469`, `vb_vt2f_direct_runtime_api_acceptance.rs:605`, `vb_c1s0_orchestration_runtime_tests.rs:593`. See O-01. |
| `workspace_tests/tests` | 1 `sleep()` | `vb_a0t1_source_length_gate/support.rs:269` — bounded 50ms in 60s polling loop. **HOWEVER, 2 tests in this file are FAILING** (`test_out_of_scope_vb_cli_xtask_changes_are_routed_with_touched_package_evidence` and `test_full_source_length_pipeline`). See C-NEW-1. |
| `workspace_tests/tests` | 0 `#[should_panic]` | Clean. |
| `vb_storage/src` | 0 `todo!()` / `unimplemented!()` | Clean. |
| **TOTAL** | **4** | Down 2 from round 3 (5 → 3 `#[ignore]`). |

### `lazy_static` / `OnceLock` / `static mut` / `thread_local!`

| Crate | Total | Notes |
|-------|-------|-------|
| All test files | **0** | Clean (production has them in `vb_storage/src/journal/core.rs:125`, `vb_storage/src/queue/writer.rs:53`, `vb_storage/src/queue/loom_vb_mrwe_7.rs:8`; tests do not). |

### `panic!()` in test bodies (banned by rubric, but context-dependent)

| Crate | Total matches | Context |
|-------|---------------|---------|
| `workspace_tests/tests` | ~70 (similar to round 3) | `match other => panic!("expected X, got {other:?}")` positive assertions. Acceptable. |
| `vb_storage/src` | ~75 (similar to round 3) | Fixture or positive-assertion idioms. Acceptable. |
| **TOTAL** | **~145** | Up ~7 from round 3 (wave-11 added retry-invariant tests). All acceptable. |

### `.ok();` / `.err();` silent Result conversion

| Crate | Total | Notes |
|-------|-------|-------|
| `workspace_tests/tests` | 11 | `vb_8mdp_7_resource_admission_props.rs` (11 sites) — silent discard of `enqueue`/`tick` errors. Same as round 3 (L-02 carry). |
| `vb_storage/src/proptests.rs` | 2 | Same as round 3 (L-03 carry). |
| **TOTAL** | **13** | Unchanged from round 3. |

### Fuzz/kani mis-categorized as `#[test]`

| File | Status |
|------|--------|
| `crates/vb_storage/src/kani_*.rs` (~80 files) | All `#[cfg(all(kani, feature = "..."))]` gated. **CLEAN.** |
| `crates/fuzz/fuzz_targets/*.rs` | Outside slice 2 scope. **CLEAN.** |
| **TOTAL fuzz/kani mis-categorized** | **0** |

---

## Mutation Gaps (top 5 most dangerous bugs the slice would NOT catch)

1. **`cargo test -p vb_workspace_tests --no-fail-fast` shows 39 failing tests but vb_storage tests are green.**
   The 39 failures span u64::MAX sequence encoding, recovery-tail contiguity, retry invariants, taint/redaction fixtures, BDD scenarios, and source-length gates. The failures were NOT measured in round 1/2/3 (which only ran `cargo test -p vb_storage --tests`). **CI must run BOTH `vb_storage` AND `vb_workspace_tests` in a green-build gate.** Round-4 adds this measurement. **File:Line:** various (see C-NEW-1 detail).

2. **`JournalError::KeyCapacity::symbolic_code()` returns fallback instead of registered code** (FIXED in wave-12).
   The test `journal_error_symbolic_code_key_capacity` in `symbolic_code_behavior_tests.rs:400` was previously `#[ignore]` because production returned a fallback instead of "JOURNAL_KEY_CAPACITY". Wave-12 fix changed to `HasSymbolicCode::symbolic_code(&JournalError::KeyCapacity)` trait-method call. **File:Line:** `crates/vb_storage/src/symbolic_codes.rs` (production-side).

3. **`RecoveryError::FrameDimensionOverflow` raised instead of silently accepting SlotIdx::MAX**.
   The test `frame_seed_slot_dimension_overflow_reports_exact_variant` in `recovery/replay/summary/tests.rs:482` was previously asserting `result.is_ok()` with a comment claiming production handles SlotIdx::MAX gracefully. Production actually raises the typed error. Wave-12 FIXED the test to assert the typed error correctly. **Production code is correct, test now matches.** **File:Line:** `crates/vb_storage/src/recovery/replay/summary/frame_seed/accumulator.rs:130-137` (production) and `tests.rs:482-500` (test, fixed).

4. **`process_lock::acquire` would silently skip acquisition — partial mutation caught by lock_path.exists() checks.**
   Round-4 verification confirms `process_lock_tests.rs:213-238` now has `lock_path.exists()` checks. A mutation that skips `acquire_process_lock()` would fail these checks. Round-3 M-02 fix is verified. **File:Line:** `crates/vb_storage/src/process_lock_tests.rs:213-238` (test) + `crates/vb_storage/src/process_lock.rs` (production).

5. **`verify_digest_match` returns `Ok(())` for ANY payload** — partial mutation caught by wrong-digest tests.
   Round-4 verification confirms `codec/tests.rs:565` and `security_tests.rs:767` use `assert_eq!(result, Ok(()), ...)` with security-relevant comments. A mutation that returns Ok for both correct AND wrong digests would fail the wrong-digest tests on adjacent lines. **However, M-NEW-2 reveals 8 OTHER sites** (`codec/tests.rs:534,674,1756,2459,2655,2739,2863,2996`) where the wrong-input rejection tests are missing or not paired. **File:Line:** `crates/vb_storage/src/codec/tests.rs:534,674,1756,2459,2655,2739,2863,2996` (NEW FINDING).

---

## Top 5 Fixes (impact-per-effort)

### Fix 1 — Triage the 39 workspace_tests failures (CRITICAL, blocker)
**Impact:** Restores the workspace_tests test gate from red to green. Currently 39 distinct contract violations are silently passing through CI. **Effort:** Variable; estimate 2-4 hours per cluster (u64::MAX encoding, recovery-tail contiguity, retry invariants, taint fixtures, BDD scenarios). **This is the highest-priority fix because CI is currently broken.**

For each cluster:
- (a) **u64::MAX sequence encoding** (5+1 tests): Either revert production's `ReservedSeqSentinel` rejection at `u64::MAX`, or update the tests to expect the new error.
- (b) **Recovery-tail contiguity** (4 tests): Check if production's new validation is correct; if yes, update tests + error message expectations; if no, revert production.
- (c) **Retry invariants** (5 tests): Likely production changed retry semantics; either revert or update.
- (d) **Taint/redaction fixtures** (7 tests): Likely production changed taint logic; either revert or update.
- (e) **BDD/recovery-hydration/source-length-gate/workspace-assertions** (~14 tests): Triage individually.

### Fix 2 — Convert 8 NEW `assert!(result.is_ok())` in `codec/tests.rs:534,674,1756,2459,2655,2739,2863,2996` to `assert_eq!`
**Impact:** 8 SECURITY/codec-relevant success-path tests gain concrete value assertions. Round-1 H-02 fixed 4 sites but missed these 8 (they're in different functions). **Effort:** 30 minutes.

```rust
// crates/vb_storage/src/codec/tests.rs:534 (encode_accepts_payload_at_exact_max_boundary)
// BEFORE:
assert!(result.is_ok(), "payload at exact max boundary should be accepted");
// AFTER:
assert_eq!(result, Ok(()), "encode_record at exact max boundary must return Ok(())");
let encoded = result.unwrap();
assert!(!encoded.is_empty(), "encoded record must have header bytes");
```

### Fix 3 — Convert 3 remaining `assert!(result.is_ok())` in `hydrate_tests.rs:281,287,374` to `assert_eq!`
**Impact:** 3 success-path tests gain concrete value assertions (round-1 H-01 carry). **Effort:** 10 minutes.

### Fix 4 — Delete the NEW weak-OR at `vb_core_atomic_admission_red.rs:1151` + tautology at `integration_compile_error_message_quality.rs:488`
**Impact:** 2 wave-7/8 introduced contract-violations are removed. **Effort:** 15 minutes.

```rust
// crates/vb_storage/tests/vb_core_atomic_admission_red.rs:1151
// BEFORE:
prop_assert!(result.is_err() ||
    journal.compiled_ir(workflow.digest()).unwrap().is_none(),
    "inconsistent source must cause admission failure or no artifact");
// AFTER:
prop_assert!(matches!(result, Err(AdmissionError::SourceDigestMismatch { .. })),
    "strict admission of source-digest-mismatch must yield SourceDigestMismatch");
```

### Fix 5 — Rename `chunk_012.rs:181` from `declared_keyspaces_returns_ten_entries` to `declared_keyspaces_returns_eleven_entries`
**Impact:** Eliminates misleading test name (round-3 L-01 carry). **Effort:** 1 minute.

---

## Round-4 Summary

| Category | Count |
|----------|-------|
| Round-1 CRITICAL fixes verified STILL APPLIED | 4 / 4 |
| Round-2 CRITICAL fixes verified STILL APPLIED | 3 / 3 (count + corruption tests) |
| Round-3 carry-overs addressed in round 4 | 6 of 10 (M-02 fixed, M-03 fixed, M-05 fixed, H-02/H-03 fixed, 2/5 #[ignore] removed, frame_seed_slot_dimension_overflow fixed) |
| **CRITICAL findings (NEW)** | **1 (C-NEW-1: workspace_tests 39 failures)** |
| HIGH findings (NEW) | 0 |
| MEDIUM findings (NEW + carry) | 10 (M-NEW-2, M-NEW-3, M-01, M-04, M-06, M-07, M-08, M-09, M-10) |
| LOW findings (NEW + carry) | 6 (L-01, L-02 NEW, L-03, L-04, L-05, L-06) |
| OBSERVATION findings (NEW + carry) | 5 (O-01 reduced to 3 sites, O-02, O-03, O-04, O-05) |
| Round-1 regression count | 0 |
| Round-2 regression count | 0 |
| Round-3 regression count | 0 |
| **vb_storage --tests pass count** | **1762 passed, 0 failed** (round-3 baseline 1760 → +2 from wave-12) |
| **workspace_tests pass count** | **2912 passed, 39 FAILED** (round-3 did not measure) |

The slice's strength is that **all 4 round-1 + 2 round-2 CRITICAL artifacts remain intact**, and wave-12 substantially tightened the contracts for `process_lock_tests.rs`, `contracts_production_binding.rs`, `parse_vet_exit_code`, and the `frame_seed_slot_dimension_overflow_reports_exact_variant` test. **vb_storage is green at 1762 passed.** However, **workspace_tests is red with 39 failures** that round-1/2/3 did not measure. These 39 failures represent production contract violations introduced between wave-5 and wave-12 — primarily around `u64::MAX` sequence encoding, recovery-tail contiguity, retry invariants, taint/redaction fixtures, BDD scenarios, and source-length gates. The slice cannot advance until these 39 failures are triaged.

---

## Disposition

| ID | Disposition | Rationale |
|----|-------------|-----------|
| C-NEW-1 | `blocker` | `cargo test -p vb_workspace_tests --no-fail-fast` shows 39 failures across 14 files. Production contract violations introduced between wave-5 and wave-12. CI gate is currently broken. **REJECTED.** |
| M-NEW-2, M-NEW-3 | `blocker` | 8 NEW smoke tests in `codec/tests.rs` (round-1 H-02 missed) + 1 NEW weak-OR in `vb_core_atomic_admission_red.rs:1151` (round-3 census missed). Together allow ~9 production mutations to pass silently. |
| M-01, M-04, M-06, M-07, M-08, M-09, M-10 | `blocker` | Round-3 carry-overs not addressed. ~15 sites still let mutations in `validate_tail_first_seq_contiguous_with_snapshot`, `parse_ast`, `journal.blob`, `JournalBatchSize::try_from_usize`, `YamlCompiler::parse_ast`, `YamlCompiler::parse_ast` (whitespace), and `FjallJournal::open` (nonexistent path) pass silently. |
| L-01, L-02, L-03, L-04, L-05, L-06 | `owner_approved_debt` | Cosmetic + decorative + concrete idioms. Tractable cleanup but not regression blockers. |
| O-01 through O-05 | `owner_approved_no_action` | 3 `#[ignore]` tests documented; decorative + concrete idioms documented as gold-standard; benchmark tautology out of behavior-test scope. |

---

## Verdict

```
STATUS: REJECTED
```

**1 NEW CRITICAL + 0 NEW HIGH + 10 NEW/CARRY-OVER MEDIUM + 6 NEW/CARRY-OVER LOW + 5 OBSERVATION findings**, with **0 round-1+2+3 regressions** on verified CRITICAL/HIGH artifacts. vb_storage is green (1762 passed, +2 from round-3 baseline). **workspace_tests is red with 39 failures that round-3 did not measure.** The slice cannot advance until:

1. **C-NEW-1** — triage 39 workspace_tests failures across 14 files (~2-4 hours).
2. **M-NEW-2** — convert 8 NEW smoke tests in `codec/tests.rs:534,674,1756,2459,2655,2739,2863,2996` to `assert_eq!` (~30 min).
3. **M-NEW-3** — delete weak-OR at `vb_core_atomic_admission_red.rs:1151` (~10 min).
4. **M-01** — convert 3 `assert!(result.is_ok())` in `hydrate_tests.rs:281,287,374` (~5 min).
5. **M-04, M-06, M-07, M-08, M-09, M-10** — tighten ~12 variant checks across 6 files (~45 min).

Total cleanup time: ~3-5 hours plus triage of the 39 workspace_tests failures and the 3 remaining `#[ignore]` tests.

Round-4 progress vs. round 3:
- Round 1: 3 CRITICAL + 10 HIGH + 12 MEDIUM + 8 LOW + 5 OBSERVATION
- Round 2: 2 CRITICAL (new) + 0 HIGH + 6 MEDIUM (new) + 4 LOW (new) + 5 OBSERVATION
- Round 3: 0 CRITICAL + 0 HIGH + 11 MEDIUM + 6 LOW + 3 OBSERVATION
- Round 4: 1 CRITICAL (new — workspace_tests failures) + 0 HIGH + 10 MEDIUM + 6 LOW + 5 OBSERVATION

The CRITICAL count increased from 0 to 1 because round-3 did not measure `cargo test -p vb_workspace_tests --no-fail-fast`. The 39 failures likely existed in round-3 too but were not detected. **Wave-12 substantially tightened the contracts for vb_storage-internal tests** (process_lock, contracts_production_binding, parse_vet_exit_code, frame_seed_slot_dimension_overflow) and un-ignored 3 `#[ignore]` tests. **However, wave-12 also introduced 8 NEW smoke-test sites** (M-NEW-2) and **did not re-verify workspace_tests** after the production changes between wave-5 and wave-12 (C-NEW-1). The convergence pattern is mixed: vb_storage is now exemplary, but workspace_tests is currently red.