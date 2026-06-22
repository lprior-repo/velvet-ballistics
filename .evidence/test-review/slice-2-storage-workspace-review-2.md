# Test Review — Slice 2 (Round 2): vb_storage + workspace_tests

**Scope:** 313 test-bearing Rust files (116 in `crates/vb_storage/`; 197 in `crates/workspace_tests/`; plus benches).
Kani harnesses (`kani_*.rs`, ~80 files) are correctly `#[cfg(kani)]`-gated proof artifacts and excluded from
behavior-test review.

**Date:** 2026-06-21
**Reviewer:** test-reviewer agent (slice 2 of 4, round 2)
**Round:** 2 of 40 — verify round-1 fixes + find NEW defects from wave-5/6/7.

## STATUS: REJECTED

Round-1 CRITICAL findings (C-01, C-02, C-03, C-04) are all STILL APPLIED — the four tautologies,
the two "match-all-outcomes" process_lock tests, and the rename for the empty-payload test are
correctly fixed. Round-1 HIGH findings (H-01 through H-10) are **NOT ADDRESSED**: 8 `assert!(result.is_ok())`
sites in `hydrate_tests.rs`, 3 in `codec/tests.rs`, 1 in `security_tests.rs`, and 2 in
`process_lock_tests.rs` still smoke-test success paths without concrete value verification. New CRITICAL
findings from wave-5/6/7: `declared_keyspaces` was expanded to 11 entries but **two tests still assert 10** —
`doctor_key_decode_tests.rs:621` and `fjall_keyspace_manifest_tests.rs:313`. Both fail in `cargo test -p
vb_workspace_tests`. The slice has **2 CRITICAL (NEW), 0 HIGH (NEW), 6 MEDIUM (NEW), 4 LOW (NEW)** plus
**8 unfixed round-1 HIGH** carry-overs that remain blockers for APPROVED status. Cannot approve until the
two CRITICAL failures and the 8 carry-over HIGH smoke-test sites are corrected.

---

## Round-1 Fix Verification Table

| Round-1 ID | File:Line | Round-1 Defect | Status | Evidence |
|------------|-----------|----------------|--------|----------|
| C-01 / H-08 | `integration_compile_error_message_quality.rs:374-378,401-405,426-430` | `assert!(result.is_ok() \|\| result.is_err())` tautology (3 sites) | **STILL APPLIED** | Lines 376, 402, 428 now use `matches!(result, Err(CompileErrors(ref errors)) if errors.iter().any(|e| matches!(e, CompileError::DepthLimit { depth, limit } if *depth > *limit && *limit == 1)))` — concrete variant + field check. Tautology deleted. |
| C-02 | `integration_runtime_storage_fault_tolerance.rs:215` | `assert!(result.is_ok() \|\| result.is_err())` tautology | **STILL APPLIED** | Line 215-218 now asserts `matches!(result, Err(ref e) if matches!(e, RuntimeError::InvalidRecoveryHydration))` — concrete error variant check with doc-comment on contract. |
| C-03 | `process_lock_tests.rs:141-181` (now 141-179) | "match-all-outcomes" `_ = other;` and `Ok(_) => {} \| Err(_) => {}` patterns (2 tests) | **STILL APPLIED** | Lines 149-152: `assert!(matches!(result, Err(JournalError::ProcessLockHeld { .. }))` — concrete rejection variant. Lines 156-178: `lock_path.exists()` before drop, `!lock_path.exists()` after drop, then `result.is_ok()` for re-open. Both SECURITY-relevant contracts now have meaningful assertions. |
| C-04 | `edge_case_tests.rs:547` | Test name `encode_rejects_zero_length_payload_serialization` contradicted `assert!(result.is_ok(), "empty payload should be accepted")` | **STILL APPLIED** | Renamed to `encode_accepts_zero_length_payload_and_round_trips` (line 547) and now performs full round-trip: encodes, asserts `!encoded.is_empty()`, decodes, asserts `decoded.bytes == vec![]`, `decoded.digest == [0u8; 32]`, `envelope.kind == RecordKind::Blob`. **Round 1 fix is exemplary.** |

**Round-1 regression count: 0.** All four CRITICAL fixes are intact and correctly scoped.

---

## Round-1 HIGH Carry-over Status (NOT FIXED in Round 1)

The 4 round-1 CRITICAL fixes landed correctly, but the 10 round-1 HIGH findings were **NOT closed**. They remain
in the codebase, unchanged from round 1. Listed below for completeness; see "Findings" for the new round-2
assessment.

| Round-1 ID | File:Line | Status | Round-2 Assessment |
|------------|-----------|--------|---------------------|
| H-01 | `hydrate_tests.rs:140,163,186,216,222,228,257,263,350` (9 sites) | NOT FIXED | Re-classified MEDIUM (was HIGH). See M-01. |
| H-02 | `codec/tests.rs:565,2813,2825,2850` (4 sites — round-1 said 3, +1 added) | NOT FIXED | Re-classified MEDIUM. See M-02. |
| H-03 | `security_tests.rs:767` | NOT FIXED | Re-classified MEDIUM. See M-03. |
| H-04 | `process_lock_tests.rs:213-228` (2 sites — `open_store_acquires_process_lock` 217, `init_keyspaces_acquires_process_lock` 225) | NOT FIXED | Re-classified MEDIUM. See M-04. |
| H-05 | `tests/chunk_004.rs:166,274`, `tests/chunk_008.rs:173,181,185`, `tests/chunk_040.rs:212,227,268` (8 sites) | NOT FIXED | Re-classified LOW. See L-01. |
| H-06 | `contracts_production_binding.rs` (15 sites) | NOT FIXED | Out of slice-2 priority; carried over. |
| H-07 | `vb_test_compile_parse_validate_behavior.rs:184-188` (`parse_rejects_whitespace_only_source`) | NOT FIXED | Re-classified MEDIUM. See M-05. |
| H-08 | (covered by C-01) | RESOLVED | — |
| H-09 | 5 `#[ignore]` behavior tests | NOT FIXED | Same 5 ignored tests remain. See OBSERVATION O-01. |
| H-10 | `contracts_production_binding.rs:282` | NOT FIXED | Out of slice-2 priority; carried over. |

**Round-1 HIGH carry-over count: 7 unfixed + 3 covered elsewhere. None regressed.**

---

## Findings (CRITICAL first)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix |
|----|-----|-----------|--------|------------------------------|------------------|
| C-05 | CRITICAL | `crates/workspace_tests/tests/doctor_key_decode_tests.rs:617-626` | `readonly_journal_declared_keyspaces_returns_ten` asserts `assert_eq!(spaces.len(), 10)` but `ReadOnlyJournal::declared_keyspaces()` returns `[&'static str; 11]` (per `crates/vb_storage/src/journal/readonly.rs:49`). **Test currently FAILS** (left=11, right=10). Wave-5/6 added `run_seq_gap` keyspace (per `chunk_012.rs:184`); this test was not updated. The test name says "ten" but production says 11 — neither matches the contract. | Delete the 11th keyspace (`run_seq_gap`) from `FjallJournal::declared_keyspaces()`. Test passes. The test asserts the wrong number and so does not catch the deletion of any non-run_seq_gap keyspace. | Rename to `readonly_journal_declared_keyspaces_returns_eleven` and change `10` to `11` in the assertion. Better: assert specific names from a list (matches the gold-standard `chunk_012.rs:186-197` pattern). |
| C-06 | CRITICAL | `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs:309-317` | `declared_keyspaces_count` asserts `assert_eq!(keyspaces.len(), 10, "declared_keyspaces must return exactly 10 entries")`. **Test currently FAILS** — same wave-5/6 root cause as C-05. Two separate tests for the same contract, both asserting the wrong count. | Same as C-05. Either test passing alone (after the fix) doesn't catch regressions on the other. | Update to `11` and assert specific keyspace names. Or factor into a single shared helper used by both tests. |
| M-01 | MEDIUM | `crates/vb_storage/src/hydrate_tests.rs:140,163,186,216,222,228,257,263,350` (9 sites) | `assert!(result.is_ok(), "matching run should succeed, got {:?}", result)` — round-1 H-01 still not fixed. 9 happy-path tests pass if `validate_snapshot_metadata`, `validate_tail_run_metadata`, etc. return `Ok(())` for ANY input. The failure-path tests on adjacent lines catch partial mutations, but the success paths don't verify the result carries any data. | Replace `validate_snapshot_metadata` with `Ok(())` for all inputs. All 9 success tests pass; the failure-path tests (e.g., line 148-155 for mismatched run) still fail. Partial mutation (delete success-path check) not caught. | Replace each with `assert_eq!(result, Ok(()))` for explicit unit comparison. |
| M-02 | MEDIUM | `crates/vb_storage/src/codec/tests.rs:565,2813,2825,2850` (4 sites, round-1 H-02 was 3) | `assert!(result.is_ok(), "correct digest should pass verification")` (line 565), and 3 in `validate_replayed_event_*` boundary tests (2813, 2825, 2850). SECURITY-relevant: a mutation to `verify_digest_match` that always returns `Ok(())` would silently break the contract. | Replace `verify_digest_match` with `Ok(())`. Test on line 565 passes; the wrong-digest test on line 569-578 still fails. Partial mutation caught only because the wrong-digest test exists. | Replace with `assert_eq!(result, Ok(()))` and assert returned values where applicable. For digest tests, also assert the returned digest bytes match `blake3::hash(payload)`. |
| M-03 | MEDIUM | `crates/vb_storage/src/security_tests.rs:767` | `assert!(result.is_ok(), "correct digest should pass verification")` — round-1 H-03 still not fixed. SECURITY test that does not verify the return value. | Replace `verify_digest_match` with `Ok(())`. Wrong-digest test on line 776 still fails (good). A partial mutation that always returns Ok for both correct AND wrong digests via two different code paths is not caught by this test alone. | Replace with `assert_eq!(result, Ok(()))`. |
| M-04 | MEDIUM | `crates/vb_storage/src/process_lock_tests.rs:213-218,220-228` (2 sites) | `open_store_acquires_process_lock` (line 217) and `init_keyspaces_acquires_process_lock` (line 225) only assert `assert!(result.is_ok())`. Function names claim "acquires process lock" but no assertion verifies a lock file was actually created. Round-1 H-04 still not fixed. | In `open_store`, skip the `acquire_process_lock()` call. Both tests pass; the `process_lock_file_is_created` test on line 181-192 still catches this (because it uses `lock_path.exists()`). But the `open_store` and `init_keyspaces` functions themselves are not directly tested for the lock contract. | After `assert!(result.is_ok())`, add `let lock_path = temp.path().join(".process.lock"); assert!(lock_path.exists(), "open_store must create .process.lock");` (copy from line 184-194 pattern). |
| M-05 | MEDIUM | `crates/workspace_tests/tests/vb_test_compile_parse_validate_behavior.rs:184-188` | `parse_rejects_whitespace_only_source` uses `assert!(result.is_err(), "whitespace-only source should fail")` — accepts any error. Round-1 H-07 still not fixed. The previous test `parse_rejects_totally_empty_source` (line 172-182) checks the message contains "empty" or "document", but this whitespace-only test does NOT check the message. | Make `YamlCompiler::parse_ast(b"   \n\n   \n")` return `Err(CompileError::Other)`. Test passes; test for empty source would still fail. | Add `let msg = result.unwrap_err().to_string(); assert!(msg.contains("empty") \|\| msg.contains("whitespace"));` |
| M-06 | MEDIUM | `crates/vb_storage/src/type_tests.rs:191-197` (line 196) | `assert!(err.is_err(), "zero should fail")` — accepts any error variant. The `JournalBatchSize::try_from_usize(0)` failure should produce a specific `JournalBatchSizeError::Zero` or similar variant; the test does not verify which variant. | Make `try_from_usize(0)` return `Err(JournalBatchSizeError::SomeOtherVariant)`. Test passes. The test does not bind to the contract that zero is rejected with a "zero" or "non-zero" error code. | Replace with `assert!(matches!(err, Err(JournalBatchSizeError::Zero)));` |
| M-07 | MEDIUM | `crates/vb_storage/src/blob_tests.rs:246-263` (line 260) | `result.is_err() \|\| result.map_or(false, \|opt\| opt.is_none())` — accepts EITHER `Err(anything)` OR `Ok(None)`. The contract is "must return a typed error for corrupt data" — the test accepts silent absence. | Make `journal.blob(digest)` return `Ok(None)` for corrupt data. Test passes silently; no error is reported to the caller. This silently downgrades the security contract. | Replace with `assert!(matches!(result, Err(JournalError::BlobDecode(_) \| JournalError::PayloadDigestMismatch { .. })));` |
| M-08 | MEDIUM | `crates/vb_storage/src/security_tests.rs:1047-1059` (line 1056) | `lock_releases_on_journal_drop` asserts `result.is_ok()` after dropping the journal. Round-1 M-11 not addressed. The test would pass if the lock was never acquired AND never released. The `process_lock_file_is_created` test on line 1029-1044 catches the "never acquired" path independently, but the SECURITY contract "lock is acquired and released" is split across two tests. | n/a (covered by line 1029-1044). | Add `let lock_path = temp.path().join(".process.lock"); assert!(!lock_path.exists(), ".process.lock must be released on drop");` before line 1056, copying from `process_lock_tests.rs:168-171`. |
| L-01 | LOW | `crates/vb_storage/src/tests/chunk_004.rs:166,274`, `chunk_008.rs:173,181,185`, `chunk_040.rs:212,227,268` (8 sites) | Round-1 H-05: `assert!(journal.is_ok(), "...")` for `FjallJournal::open` and `open_store`. Re-classified LOW because the chunk test files are part of a tightly-coupled `chunk_NNN.rs` test suite where each chunk test verifies a complete round-trip and the `is_ok()` is decorative. | n/a — the round-trip assertions that follow (e.g., `chunk_008.rs:186`: `assert_eq!(FjallJournal::declared_keyspaces().len(), 10)` — wait, this is now 11 and stale). | Acceptable as decorative if accompanied by a write-read round-trip. Note `chunk_008.rs:186` asserts `== 10` but production returns 11 — see C-06 (the chunk test version of the same bug). |
| L-02 | LOW | `crates/workspace_tests/tests/vb_8mdp_7_resource_admission_props.rs:256,276,282,301,654,673,678,913,914,930,965` (11 sites) | `shard.enqueue(...).ok();` and `shard.tick().ok();` — silent error suppression in property test setup. The `if let Some(ok) = tick_ok { if !ok { break; } }` pattern (line 914-919) shows the author knew the result mattered but chose to swallow it. Acceptable for "expected-to-succeed" enqueue setup, but masks regressions if `enqueue` returns `Err` for legitimate inputs. | Make `shard.enqueue(submit_command(0))` return `Err(ShardError::QueueFull)`. All tests still pass — the `.ok()` swallows it. | Use `.expect("setup enqueue must succeed for valid input")` for known-good setup inputs. |
| L-03 | LOW | `crates/vb_storage/src/proptests.rs:455,469` (2 sites) | `recover_runtime_summary(&journal1, run).ok();` — silent error suppression in proptest setup. The follow-up `prop_assert_eq!` on `summary1` and `summary2` would still fail if both return `None`, so partial mutation caught. | Make `recover_runtime_summary` return `Ok(Summary::default())` for all inputs. Both summary comparisons still pass — the test never checks that summary has actual fields. | Use `.expect("summary recovery must succeed in proptest setup")`. |
| L-04 | LOW | `crates/workspace_tests/tests/vb_y1zq_boundary_inventory_properties_v2.rs:565,599,611` (3 sites) | `prop_assert!(result.is_err(), "...")` — accepts any error variant for `validate_evidence_reference_bytes` rejection cases (trailing component, empty suffix, missing vb- prefix). The contract is "rejects with specific ValidationError variant"; the test accepts any Err. | Make `validate_evidence_reference_bytes` return `Err(EvidenceReferenceError::Other)`. All 3 prop tests pass. | Convert to `prop_assert!(matches!(result, Err(EvidenceReferenceError::InvalidSuffix | EvidenceReferenceError::MissingPrefix { .. })));` |
| O-01 | OBSERVATION | `vb_qi37_25_quality_gates.rs:250`, `vb_c1s0_orchestration_runtime_tests.rs:593`, `vb_test_runtime_lifecycle_state_behavior.rs:469`, `vb_vt2f_direct_runtime_api_acceptance.rs:605`, `vb_njju_mutation_fuzz_property_closure.rs:224` (5 sites, same as round-1 H-09) | `#[ignore]` on behavior tests, all documented as "pre-existing issue" or "pending GAP". Same status as round-1. | Mark them all as `#[ignore]` permanently — tests pass by not running. Production regressions in those contract areas would not be caught. | Open beads for each ignored test to track closure. The closure plan (fix or remove) was not executed in the round-1 cycle. |
| O-02 | OBSERVATION | `crates/workspace_tests/tests/vb_test_cli_storage_io_behavior.rs:346-822+` (15 sites) | `assert!(result.is_ok(), "events_for_run must succeed")` followed by `let events = result.expect("events ok")` and concrete `matches!(terminal, Some(JournalEvent::RunFinished { .. }))` checks. This is the same **decorative + concrete** pattern as `journal_event_tests.rs:370` (gold standard per round-1 O-03). The `is_ok()` is decorative; the `matches!` is the real check. | n/a — the concrete check makes the test meaningful. | Acceptable as decorative + concrete. Document this idiom in test module header. |
| O-03 | OBSERVATION | `crates/workspace_tests/tests/postcard_envelope_wire_tests.rs:545,575,602,684,713,738` (6 sites) | Round-1 L-02 re-encountered: `prop_assert!(result.is_err(), "...")` without specific variant check. Same pattern as M-02 round-1. | Same as M-02 round-1. | Convert each to `prop_assert!(matches!(result, Err(PostcardError::SpecificVariant)));` |
| O-04 | OBSERVATION | `crates/vb_storage/src/codec/tests/kill_kind_admission.rs`, `codec/tests/replay_integrity.rs` (wave-5/6/7 new test files, 484+362 = 846 lines) | These are the new codec tests added in wave-5/6/7. They are EXEMPLARY: every assertion uses `matches!` with specific variants AND field checks (`expected == EventSeq::new(3) && actual == EventSeq::new(5)`). Zero `assert!(result.is_ok())` smoke tests. The new code's quality is the gold standard for this slice. | n/a — exemplary. | Out of scope. |
| O-05 | OBSERVATION | `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs` (wave-5/6/7, 2078 lines, 28 #[test] functions) | Large new test file (wave-6 commit `906d96ad6`). Every assertion uses `assert_eq!` with concrete values: `result.verification.gate_count == 15`, `result.verification.durable == true`, `result.digest == workflow.digest()`. Zero banned `is_ok()` patterns. Excellent. | n/a — exemplary. | Out of scope. |

---

## Pattern Census

### `assert!(...is_ok()) / assert!(...is_err())` (BANNED in behavior assertions)

| Crate | Total matches | Top files (round 2) |
|-------|---------------|---------------------|
| `vb_storage/src` | 28 | `hydrate_tests.rs` (9 — same as round 1), `codec/tests.rs` (4 — round-1 was 3, +1 at 2850), `security_tests.rs` (2 — 767, 1072), `tests/chunk_040.rs` (3), `tests/chunk_008.rs` (3), `tests/chunk_004.rs` (2), `process_lock_tests.rs` (1), `recovery/recovery_unit_tests.rs` (1), `recovery/replay/summary/tests.rs` (1), `type_tests.rs` (1), `codec/tests/replay_integrity.rs` (1) |
| `vb_storage/tests` | 0 | (none in `crates/vb_storage/tests/`) |
| `workspace_tests/tests` | 113 | `integration_validate_yaml_parsing.rs` (19), `vb_test_cli_storage_io_behavior.rs` (17, mostly decorative + concrete), `runtime_version_barrier_tests.rs` (15, mostly decorative + concrete), `vb_test_core_workflow_slot_behavior.rs` (13, mostly decorative + concrete), `doctor_storage_scan_decode_tests.rs` (13), `vb_8mdp_7_resource_admission_props.rs` (10, prop_assert), `vb_test_compile_parse_validate_behavior.rs` (9), etc. |
| `workspace_tests/benches` | 12 | Same as round 1. |
| **TOTAL** | **~153** | (Down ~17 from round 1's ~170, but round-1 said 30 in vb_storage and now 28 — net change in vb_storage minimal. workspace_tests down from 113 to 113 — same total. The actual delta is mostly cosmetic line shifts.) |

### Tautology assertions (`is_ok() || is_err()` or vice-versa)

| File:Line | Status |
|-----------|--------|
| All 4 round-1 tautology sites | **DELETED** (round-1 fix verified above) |
| `crates/vb_storage/src/blob_tests.rs:260` | **NEW FINDING** — `result.is_err() \|\| result.map_or(false, \|opt\| opt.is_none())` accepts EITHER error OR silent absence. Logically weaker than tautology but functionally similar — see M-07. |
| **TOTAL** | **0 tautologies + 1 weak-OR** |

### `let _ = ...` (silent error suppression)

| Crate | Total | Notes |
|-------|-------|-------|
| `vb_storage/src` | 6 | `bdd_validation_tests.rs` parallel has ~28, `index_tests.rs` (7 — `item.key()` discarding — fixture iteration), `tests/chunk_*.rs` (4 — `builder.try_push()` discarding capacity check), `vb_core_atomic_admission_red.rs` (1), `kani_*` (3 — proof harnesses). |
| `workspace_tests/tests` | ~80 | Same as round 1: `bdd_validation_tests.rs:1364-1402` (37 — `let _ = ValidationError::VariantName` — variant-existence check, **acceptable**), `timer_deadline_primitive_tests.rs` (~50 — fixture), `ipc_flag_matrix_tests.rs` (~6 — socket cleanup). |
| **TOTAL** | **~86** | Down ~4 from round 1. Most are acceptable (variant existence, fixture construction). |

### `#[ignore]` / `#[should_panic]` / `sleep(` / `todo!()` / `unimplemented!()`

| Crate | Total | Notes |
|-------|-------|-------|
| `workspace_tests/tests` | 5 `#[ignore]` | **SAME as round 1** — no progress. |
| `workspace_tests/tests` | 1 `sleep()` | `vb_a0t1_source_length_gate/support.rs:269` — bounded 50ms in 60s polling loop. Acceptable. |
| `workspace_tests/tests` | 0 `#[should_panic]` | Clean. |
| **TOTAL** | 6 | Unchanged from round 1. |

### `lazy_static` / `OnceLock` / `static mut` / `thread_local!`

| Crate | Total | Notes |
|-------|-------|-------|
| All test files | **0** | Clean (production has them; tests do not). |

### `panic!()` in test bodies

| Crate | Total matches | Context |
|-------|---------------|---------|
| `workspace_tests/tests` | 68 | Same as round 1 — `match other => panic!("expected X, got {other:?}")` positive assertions. Acceptable. |
| `vb_storage/src` | ~70 | `proptests.rs` (~30 — `let Ok(...) else { panic!(...) }` for fixture construction), `queue/tests.rs` (~10), `journal/tests.rs` (~10), etc. Mostly fixture or positive-assertion idioms. |
| **TOTAL** | ~138 | Up ~70 from round 1 due to wave-5/6/7 proptest additions. All acceptable as fixture/positive assertions. |

### `.ok();` / `.err();` silent Result conversion

| Crate | Total | Notes |
|-------|-------|-------|
| `workspace_tests/tests` | 11 | `vb_8mdp_7_resource_admission_props.rs` (11 sites) — silent discard of `enqueue`/`tick` errors. New finding L-02. |
| `vb_storage/src/proptests.rs` | 2 | New finding L-03. |
| **TOTAL** | 13 | Round 1 missed these. New observation. |

### Fuzz/kani mis-categorized as `#[test]`

| File | Status |
|------|--------|
| `crates/vb_storage/src/kani_*.rs` (~80 files) | All `#[cfg(all(kani, feature = "..."))]` gated. CLEAN. |
| **TOTAL** | **0** |

---

## Mutation Gaps (top 5 most dangerous bugs the slice would NOT catch)

1. **`run_seq_gap` keyspace deleted from `FjallJournal::declared_keyspaces`.** Two tests assert `len() == 10` (`doctor_key_decode_tests.rs:621`, `fjall_keyspace_manifest_tests.rs:313`); both currently FAIL because production has 11. After fixing them to assert 11, deleting `run_seq_gap` would pass both tests. The tests do NOT verify which 11 keyspaces exist — `chunk_012.rs:186-197` is the only test that does (and it asserts 11 with specific names, so it would catch the deletion). **File:Line:** `crates/vb_storage/src/journal/core.rs:137` `pub const fn declared_keyspaces() -> [&'static str; 11]`.

2. **`validate_snapshot_metadata` accepts any input (delete the matching-run check).** The 9 `assert!(result.is_ok())` in `hydrate_tests.rs:140,163,186,216,222,228,257,263,350` would all pass. The error-path tests (lines 144-156, 167-178, etc.) would still fail, so partial mutation (delete success-path check, keep failure-path check) is not caught. But if both paths are simplified to always-Ok or always-Err, no test catches it. **File:Line:** `crates/vb_storage/src/recovery/hydrate.rs::validate_snapshot_metadata` and friends.

3. **`process_lock::acquire` becomes a no-op for `open_store` / `init_keyspaces`.** `open_store_acquires_process_lock` (process_lock_tests.rs:217) and `init_keyspaces_acquires_process_lock` (process_lock_tests.rs:225) only check `is_ok()`. The `process_lock_file_is_created` test (line 181-192) catches some mutations but not all (e.g., if `open_store` is a no-op and the lock file is created by some other path). **File:Line:** `crates/vb_storage/src/lib.rs::open_store` and `crates/vb_storage/src/lib.rs::init_keyspaces`.

4. **`verify_digest_match` returns `Ok(())` for ANY payload.** `codec/tests.rs:565` and `security_tests.rs:767` only check `is_ok()`. The wrong-digest tests on adjacent lines still fail, so partial mutation caught. But if both correct and wrong digests are accepted via two different code paths that both return `Ok`, no test catches it. **File:Line:** `crates/vb_storage/src/codec/digest.rs::verify_digest_match`.

5. **`journal.blob(digest)` returns `Ok(None)` for corrupt data.** `blob_tests.rs:260` accepts EITHER `Err(_) | Ok(None)`. A mutation that silently downgrades "corrupt blob" to "blob not present" would pass the test, hiding a SECURITY-relevant failure mode (silent data corruption). **File:Line:** `crates/vb_storage/src/journal/blob_reader.rs::blob`.

---

## Top 5 Fixes (impact-per-effort)

### Fix 1 — Fix the 2 broken `declared_keyspaces` tests (CRITICAL, blocker)
**Impact:** 2 tests pass; the slice no longer has failing tests in CI. **Effort:** 5 minutes.
```rust
// crates/workspace_tests/tests/doctor_key_decode_tests.rs:617-626
// BEFORE:
fn readonly_journal_declared_keyspaces_returns_ten() {
    let spaces = ReadOnlyJournal::declared_keyspaces();
    assert_eq!(spaces.len(), 10);
    ...
}
// AFTER:
fn readonly_journal_declared_keyspaces_returns_eleven() {
    let spaces = ReadOnlyJournal::declared_keyspaces();
    assert_eq!(spaces.len(), 11, "11th keyspace run_seq_gap added in wave-5/6");
    let names: Vec<&str> = spaces.to_vec();
    assert!(names.contains(&"run_seq_gap"), "must include wave-5/6 run_seq_gap");
}

// crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs:309-317
// Same fix: 10 → 11, plus name check.
```

### Fix 2 — Convert 9 `assert!(result.is_ok())` in `hydrate_tests.rs` to `assert_eq!(result, Ok(()))`
**Impact:** 9 happy-path tests gain concrete value assertions. **Effort:** 20 minutes.
```rust
// crates/vb_storage/src/hydrate_tests.rs:140
// BEFORE:
assert!(result.is_ok(), "matching run should succeed, got {:?}", result);
// AFTER:
assert_eq!(result, Ok(()), "matching run must return Ok(())");
```

### Fix 3 — Convert 4 `assert!(result.is_ok())` in `codec/tests.rs` digest/validation paths
**Impact:** 4 SECURITY-relevant tests gain concrete assertions. **Effort:** 15 minutes.

### Fix 4 — Tighten `process_lock_tests.rs:213-228` and `security_tests.rs:1047-1059` lock-contract tests
**Impact:** 2+1 tests verify the full "acquire + release" lifecycle, not just "open succeeds". **Effort:** 15 minutes.

### Fix 5 — Triage the 5 `#[ignore]` tests in workspace_tests
**Impact:** 5 behavior tests either get fixed (preferred) or removed. **Effort:** Variable; depends on contract. **Already a carry-over from round 1.**

---

## Round-2 Summary

| Category | Count |
|----------|-------|
| Round-1 CRITICAL fixes verified STILL APPLIED | 4 / 4 |
| Round-1 HIGH carry-overs still unfixed | 7 |
| **CRITICAL findings (NEW)** | **2** |
| HIGH findings (NEW) | 0 |
| MEDIUM findings (NEW) | 6 (M-01 through M-08; some are round-1 carry-overs re-classified) |
| LOW findings (NEW) | 4 |
| OBSERVATION findings (NEW) | 5 |
| Round-1 regression count | 0 |

The slice is **structurally better than round 1** — round-1 CRITICAL fixes all held, wave-5/6/7 new tests
(`kill_kind_admission.rs`, `replay_integrity.rs`, `vb_2bok_durability_gate_tests.rs`) are exemplary and use
the gold-standard `matches!(result, Err(SpecificVariant{...}))` pattern. The remaining defects are:

1. **2 CRITICAL test failures** in CI (declared_keyspaces count drift).
2. **6 MEDIUM smoke-test sites** (round-1 H-01 through H-04 not closed) — these remain blockers.
3. **5 #[ignore] tests** — round-1 H-09 carry-over.

---

## Disposition

| ID | Disposition | Rationale |
|----|-------------|-----------|
| C-05, C-06 | `blocker` | Tests currently FAIL in CI; wave-5/6 added 11th keyspace and these tests were not updated. **REJECTED.** |
| M-01 through M-08 | `blocker` | Round-1 H-01 through H-04 carry-overs re-classified MEDIUM. ~17 sites still use `assert!(result.is_ok())` smoke tests; would let mutations in `validate_snapshot_metadata`, `verify_digest_match`, `process_lock::acquire` pass silently. |
| L-01 through L-04 | `owner_approved_debt` | Smoke tests with decorative role OR silent `.ok()` chains in property-test setup. Not immediately blocking but track as debt. |
| O-01 through O-05 | `owner_approved_no_action` | Carried-over `#[ignore]` (O-01); decorative + concrete pattern in `vb_test_cli_storage_io_behavior.rs` (O-02 — gold-standard idiom); re-encountered M-02 pattern (O-03); wave-5/6/7 new tests exemplary (O-04, O-05). |

---

## Verdict

```
STATUS: REJECTED
```

**2 NEW CRITICAL + 6 NEW MEDIUM findings**, with **0 round-1 regressions** (the 4 round-1 CRITICAL fixes
held) but **7 round-1 HIGH carry-overs** still unfixed. The slice cannot advance until:

1. **C-05 + C-06** — fix the 2 broken `declared_keyspaces` count tests (5 min).
2. **M-01 + M-02 + M-03 + M-04** — convert ~17 `assert!(result.is_ok())` smoke tests to `assert_eq!(result, Ok(()))` or concrete value checks (~50 min).
3. **M-05 + M-06 + M-07** — tighten variant checks for whitespace rejection, zero-rejection, and corrupt-blob (15 min).
4. **M-08** — strengthen `lock_releases_on_journal_drop` with concrete `lock_path.exists()` check (5 min).

Total cleanup time: ~75 minutes plus triage of the 5 `#[ignore]` tests.

The slice's strengths are notable: round-1 CRITICAL fixes held, wave-5/6/7 added excellent new test files
(`vb_2bok_durability_gate_tests.rs`, `codec/tests/kill_kind_admission.rs`, `codec/tests/replay_integrity.rs`)
that demonstrate the gold-standard pattern, and `bdd_validation_tests.rs` (62 BDD scenarios) remains
exemplary. The remaining defects are concentrated in pre-wave-5 test files that were not updated during
the round-1 fix cycle.
