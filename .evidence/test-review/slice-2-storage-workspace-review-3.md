# Test Review — Slice 2 (Round 3): vb_storage + workspace_tests

**Scope:** 313 test-bearing Rust files (116 in `crates/vb_storage/`; 197 in `crates/workspace_tests/`; plus benches).
Kani harnesses (`kani_*.rs`, ~80 files) are correctly `#[cfg(kani)]`-gated proof artifacts and excluded from
behavior-test review.

**Date:** 2026-06-21
**Reviewer:** test-reviewer agent (slice 2 of 4, round 3)
**Round:** 3 of 40 — verify round-2 fixes + find NEW defects from wave-7/8/9.

## STATUS: REJECTED

Round-2 CRITICAL findings (C-05, C-06) — both `declared_keyspaces` count tests now assert 11 — are
STILL APPLIED. Round-2 CRITICAL artifacts (the 3 corruption tests in `journal/tests.rs:1889,1931,1976`)
are intact and passing. Round-1 CRITICAL fixes (C-01, C-02, C-03, C-04) hold with no regression.
vb_storage `--tests` reports **1760 passed, 0 failed** (verified via `cargo test -p vb_storage --tests`).
Round-1 HIGH carry-overs (H-01, H-02, H-03) are now SUBSTANTIALLY ADDRESSED — most sites converted
from `assert!(result.is_ok(), ...)` to `assert_eq!(result, Ok(()), ...)`. Round-3 review surfaces:
**0 NEW CRITICAL**, **0 NEW HIGH**, **10 NEW/CARRY-OVER MEDIUM**, **6 NEW/CARRY-OVER LOW**, **3 OBSERVATION**.
Most defects are concentrated in the round-1 carry-overs (H-04 process_lock, H-06 contracts_production_binding,
H-07 compile_parse, M-04..M-11) plus **one NEW MEDIUM tautology** (`integration_compile_error_message_quality.rs:488`)
and **one NEW LOW stale test name** (`chunk_012.rs:181` says "ten" but asserts 11). Cannot advance to
APPROVED until the MEDIUM smoke-test sites and the new tautology are tightened.

---

## Round-1 + Round-2 Fix Verification Table

| Round | ID | File:Line | Defect | Status | Evidence |
|-------|----|-----------|--------|--------|----------|
| 1 | C-01 / H-08 | `integration_compile_error_message_quality.rs:374-378,401-405,426-430` | `assert!(result.is_ok() \|\| result.is_err())` tautology (3 sites) | **STILL APPLIED** | Lines 375-378, 402-405, 427-430 use `matches!(result, Err(CompileErrors(ref errors)) if errors.iter().any(|e| matches!(e, CompileError::DepthLimit/SequenceLimit/ScalarLimit{...})))` — concrete variant + field check. **0 round-2 regressions.** |
| 1 | C-02 | `integration_runtime_storage_fault_tolerance.rs:215-218` | `assert!(result.is_ok() \|\| result.is_err())` tautology | **STILL APPLIED** | Lines 215-218: `matches!(result, Err(ref e) if matches!(e, RuntimeError::InvalidRecoveryHydration))` — concrete variant check with contract doc-comment. |
| 1 | C-03 | `process_lock_tests.rs:141-179` (round-1 was 141-181) | "match-all-outcomes" with `_ = other;` and `Ok(_) => {} \| Err(_) => {}` (2 tests) | **STILL APPLIED** | Lines 149-152: `assert!(matches!(result, Err(JournalError::ProcessLockHeld { .. }))` — concrete rejection variant. Lines 156-178: `lock_path.exists()` while open, `!lock_path.exists()` after drop, `result.is_ok()` for re-open. SECURITY contract is now meaningful. |
| 1 | C-04 | `edge_case_tests.rs:547` | Test name `encode_rejects_zero_length_payload_serialization` contradicted `assert!(result.is_ok(), "empty payload should be accepted")` | **STILL APPLIED** | Renamed to `encode_accepts_zero_length_payload_and_round_trips` (line 547). Now performs full round-trip: encodes → asserts `!encoded.is_empty()` → decodes → asserts `decoded.bytes == vec![]`, `decoded.digest == [0u8; 32]`, `envelope.kind == RecordKind::Blob`. |
| 2 | C-05 | `workspace_tests/tests/doctor_key_decode_tests.rs:617-630` | `readonly_journal_declared_keyspaces_returns_ten` asserted `len() == 10` (wave-5/6 added `run_seq_gap` keyspace making it 11) | **STILL APPLIED** | Test renamed to `readonly_journal_declared_keyspaces_returns_eleven` (line 618). Line 621-625 asserts `assert_eq!(spaces.len(), 11, "declared_keyspaces must return exactly 11 entries (10 historical + run_seq_gap from wave-5/6)")`. Lines 627-630 also assert `names.contains(&"run_seq_gap")`. |
| 2 | C-06 | `workspace_tests/tests/fjall_keyspace_manifest_tests.rs:309-317` | `declared_keyspaces_count` asserted `len() == 10` (same wave-5/6 root cause as C-05) | **STILL APPLIED** | Line 312-316: `assert_eq!(keyspaces.len(), 11, "declared_keyspaces must return exactly 11 entries (10 historical + run_seq_gap from wave-5/6)")`. Plus a sibling test `declared_keyspaces_contains_required_names` (line 320-343) explicitly enumerates the 11 required names — much stronger contract than round 1's "10". |
| 2 | artifacts | `vb_storage/src/journal/tests.rs:1889,1931,1976` (round-2 brief cited 1889,1968,2002 — actual lines: 1889,1931,1976) | 3 corruption tests for `events_for_run` rejection of corrupt latest snapshot (BadMagic, PayloadDigestMismatch, PostcardDecodeFailed) | **STILL APPLIED** | All 3 tests present, all PASSING (4 corruption-related tests passed when filtered). Lines 1889-1928 (`events_for_run_rejects_corrupt_latest_snapshot_before_skipping_events` → BadMagic), 1931-1973 (`events_for_run_rejects_latest_snapshot_payload_digest_mismatch_before_tail_replay` → PayloadDigestMismatch), 1976-2007 (`events_for_run_rejects_latest_snapshot_postcard_decode_failure_before_tail_replay` → PostcardDecodeFailed). Each test uses `matches!(result, Err(SpecificVariant))` — concrete variant check. |
| 2 | artifacts | `cargo test -p vb_storage --tests` | Test pass count | **STILL APPLIED** | `cargo test: 1760 passed (36 suites, 39.06s)` — 0 failed. Round-2 baseline of 1760 maintained. |

**Round-1 regression count: 0. Round-2 regression count: 0.**

---

## Round-1 HIGH Carry-over Status (round-3 verification)

The 4 round-1 CRITICAL fixes and 2 round-2 CRITICAL fixes held. The 10 round-1 HIGH findings have been
**PARTIALLY ADDRESSED**: H-01 (hydrate_tests.rs), H-02 (codec/tests.rs), H-03 (security_tests.rs) all
substantially tightened to `assert_eq!(result, Ok(()), ...)` with security-relevant comments. H-04
(process_lock_tests.rs:213-228), H-06 (contracts_production_binding.rs), H-07 (compile_parse_validate)
remain unchanged.

| Round-1 ID | File:Line | Status | Round-3 Assessment |
|------------|-----------|--------|---------------------|
| H-01 | `hydrate_tests.rs:140,163,186,216,222,228,257,263,350` | **MOSTLY FIXED** | 6 sites converted to `assert_eq!(result, Ok(()), ...)` at lines 142, 169, 196, 230, 240, 250. **3 sites remain** at lines 281, 287, 374 (in `validate_tail_first_seq_contiguous_with_snapshot` and `validate_snapshot_recovery_inputs` functions which return `Result<(), _>`). See M-01. |
| H-02 | `codec/tests.rs:565,2813,2825,2850` | **FULLY FIXED** | All 4 sites now `assert_eq!(result, Ok(()), "...")` with explicit boundary comments. Lines 565-569, 2817-2821, 2833-2837 are concrete unit comparisons. |
| H-03 | `security_tests.rs:767` | **FULLY FIXED** | Line 767-771 now: `assert_eq!(result, Ok(()), "SECURITY: correct digest must pass verification (and must NOT silently Ok on wrong digest — see rejects_wrong test)")` — explicit unit comparison with security rationale in the message. |
| H-04 | `process_lock_tests.rs:213-228` | **NOT FIXED** | `open_store_acquires_process_lock` (line 213-218) and `init_keyspaces_acquires_process_lock` (line 220-228) still use `assert!(result.is_ok(), ...)` smoke tests. Function names claim "acquires process lock" but no `lock_path.exists()` check. Re-classified MEDIUM. See M-02. |
| H-05 | `tests/chunk_004.rs:166,274`, `chunk_008.rs:173,181,185`, `chunk_040.rs:212,227,268` | **NOT FIXED** | All 8 sites still `assert!(journal.is_ok(), ...)`. Re-classified LOW (decorative + concrete, paired with explicit `declared_keyspaces().len() == 11` checks at chunk_008.rs:188 and chunk_004.rs:175). |
| H-06 | `contracts_production_binding.rs` (15 sites) | **NOT FIXED** | 21 `is_err()` sites remain (round-2 said 15 — now 21 in this round's census). Re-classified MEDIUM. See M-03. |
| H-07 | `vb_test_compile_parse_validate_behavior.rs:184-188` (`parse_rejects_whitespace_only_source`) | **NOT FIXED** | Line 187 still `assert!(result.is_err(), "whitespace-only source should fail")` without variant check. Re-classified MEDIUM. See M-04. |
| H-09 | 5 `#[ignore]` behavior tests | **NOT FIXED** | Same 5 ignored tests remain: `vb_qi37_25_quality_gates.rs:250`, `vb_c1s0_orchestration_runtime_tests.rs:593`, `vb_test_runtime_lifecycle_state_behavior.rs:469`, `vb_vt2f_direct_runtime_api_acceptance.rs:605`, `vb_njju_mutation_fuzz_property_closure.rs:224`. See O-01. |
| H-10 | `contracts_production_binding.rs:282` | **NOT FIXED** | `assert!(parse_vet_exit_code(0).is_ok())` — `parse_vet_exit_code(0)` returns `Result<i32, String>` but the test only checks `is_ok()`, not that the value is `0`. Re-classified MEDIUM. See M-05. |

**Round-1 HIGH carry-over status: 3 of 10 addressed (H-02, H-03 fully; H-01 mostly), 7 remain.**

---

## Findings (CRITICAL first)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix |
|----|-----|-----------|--------|------------------------------|------------------|
| M-01 | MEDIUM | `crates/vb_storage/src/hydrate_tests.rs:281,287,374` (3 sites — was 9 in round 2, 6 were fixed to `assert_eq!`) | `assert!(result.is_ok(), "seq 6 must be contiguous...")`, `assert!(result.is_ok(), "empty tail must succeed...")`, `assert!(result.is_ok(), "contiguous tail should validate...")` — smoke tests on `validate_tail_first_seq_contiguous_with_snapshot` and `validate_snapshot_recovery_inputs`, both of which return `Result<(), SnapshotRecoveryInputViolation>`. Round-1 H-01 was 9 sites; 6 were fixed to `assert_eq!`, 3 missed (they're inside later test functions added in wave-6/7). | Replace `validate_tail_first_seq_contiguous_with_snapshot` with `Ok(())` always. All 3 tests pass; the `validate_tail_first_seq_contiguous_rejects_gap` test on line 291-307 still fails, so partial mutation caught. But if both paths are simplified to always-Ok, no test catches it. | Replace each with `assert_eq!(result, Ok(()), "...")`. |
| M-02 | MEDIUM | `crates/vb_storage/src/process_lock_tests.rs:213-228` (2 sites — same as round-1 H-04) | `open_store_acquires_process_lock` (line 217) and `init_keyspaces_acquires_process_lock` (line 225) only assert `assert!(result.is_ok())`. Function names claim "acquires process lock" but no assertion verifies a lock file was actually created. Round-1 H-04 still not fixed. | In `open_store`, skip the `acquire_process_lock()` call. Both tests pass; the `process_lock_file_is_created` test on line 181-192 catches the lock file absence. But `open_store` and `init_keyspaces` themselves are not directly tested for the lock contract. | After `assert!(result.is_ok())`, add `let lock_path = temp.path().join(".process.lock"); assert!(lock_path.exists(), "open_store must create .process.lock");` (copy from line 184-194 pattern). |
| M-03 | MEDIUM | `crates/workspace_tests/tests/contracts_production_binding.rs:170-175,218-222,270-273,287-290,660-661` (21 sites — round-2 said 15, now 21) | `assert!(parse_schema_version("").is_err())`, `assert!(ContractKind::parse("bogus").is_err())`, `assert!(compare_semver("1.0", "1.0.0").is_err())`, `assert!(parse_vet_exit_code(1).is_err())` — `is_err()` accepts any Err variant. Round-1 H-06 still not fixed. The contract requires specific MISSING_SCHEMA_VERSION, INVALID_VERSION, INVALID_EXIT_CODE error codes; a mutation that returned `Err(ContractError::Unknown)` for all invalid inputs would silently pass. | Change `parse_schema_version` to return `Err(ContractError::Unknown(""))` for all invalid inputs. The 6 invalid-input tests on lines 170-175 pass. The follow-up test on line 180-181 (`err.to_string() == "MISSING_SCHEMA_VERSION"`) catches this only for empty-string case. For "abc", "v1.0.0", etc., no error-code check exists. | Replace each `assert!(...is_err())` with `let err = ....unwrap_err(); assert!(err.to_string().contains("MISSING_SCHEMA_VERSION") \|\| err.to_string().contains("INVALID_VERSION"));` |
| M-04 | MEDIUM | `crates/workspace_tests/tests/vb_test_compile_parse_validate_behavior.rs:184-188` (`parse_rejects_whitespace_only_source`) | `assert!(result.is_err(), "whitespace-only source should fail")` — accepts any error. Round-1 H-07 still not fixed. The previous test `parse_rejects_totally_empty_source` (line 172-182) checks the message contains "empty" or "document", but this whitespace-only test does NOT check the message. | Make `YamlCompiler::parse_ast(b"   \n\n   \n")` return `Err(CompileError::Other)`. Test passes; the empty-source test would still fail. | Add `let msg = result.unwrap_err().to_string(); assert!(msg.contains("empty") \|\| msg.contains("whitespace"));` |
| M-05 | MEDIUM | `crates/workspace_tests/tests/contracts_production_binding.rs:282` | `assert!(parse_vet_exit_code(0).is_ok())` — `parse_vet_exit_code(0)` is the success path; the only assertion is `is_ok()`. The function returns `Result<i32, String>` — but the test does not check that the returned `i32` is `0` (the actual contract value). Round-1 H-10 still not fixed. | Make `parse_vet_exit_code` always return `Ok(42)`. Test passes. | Replace with `assert_eq!(parse_vet_exit_code(0), Ok(0));` |
| M-06 | MEDIUM | `crates/vb_storage/src/blob_tests.rs:246-263` (line 260) | `result.is_err() \|\| result.map_or(false, \|opt\| opt.is_none())` — accepts EITHER `Err(anything)` OR `Ok(None)`. The contract is "must return a typed error for corrupt data"; the test accepts silent absence. Round-2 M-07 still not fixed. | Make `journal.blob(digest)` return `Ok(None)` for corrupt data. Test passes silently; no error is reported to the caller. SECURITY-relevant: silent data corruption. | Replace with `assert!(matches!(result, Err(JournalError::BlobDecode(_) \| JournalError::PayloadDigestMismatch { .. })));` |
| M-07 | MEDIUM | `crates/vb_storage/src/recovery/recovery_unit_tests.rs:796-805` (line 800) | `assert!(result.is_err());` immediately followed by `assert!(matches!(result.unwrap_err(), RecoveryError::NoRecoveryData { .. }))`. The first assertion is decorative — the second is the real check. Round-1 M-04 still not fixed. | n/a — the second assertion is concrete. | Remove the redundant `assert!(result.is_err())` on line 800. |
| M-08 | MEDIUM | `crates/vb_storage/src/type_tests.rs:191-197` (line 196) | `assert!(err.is_err(), "zero should fail")` — accepts any error variant. The `JournalBatchSize::try_from_usize(0)` failure should produce a specific `JournalBatchSizeError::Zero` or similar variant; the test does not verify which variant. Round-2 M-06 still not fixed. | Make `try_from_usize(0)` return `Err(JournalBatchSizeError::SomeOtherVariant)`. Test passes. | Replace with `assert!(matches!(err, Err(JournalBatchSizeError::Zero)));` |
| M-09 | MEDIUM | `crates/workspace_tests/tests/doctor_storage_scan_decode_tests.rs:1266-1282` (line 1275,1278) | `parse_decode_error_invalid_keyspace_path` uses `assert!(result.is_err(), "expected error opening nonexistent path")` followed by `match result { Err(JournalError::Fjall(_)) => {} \| Err(_) => {} \| Ok(_) => panic! }`. Accepts any Err variant for the keyspace path case. Round-2 M-07 (line 1278) still not fixed. The contract is "must return typed error", not "must return ANY error". | Make `FjallJournal::open` return `Err(JournalError::ProcessLockIo { ... })` for nonexistent path. Test passes silently. | For line 1275: `assert!(matches!(result, Err(JournalError::Fjall(_) \| JournalError::PathNotFound { .. })));` |
| M-10 | MEDIUM | `crates/workspace_tests/tests/integration_compile_error_message_quality.rs:479-488` (NEW FINDING — line 488) | `let is_ok = result.is_ok(); let errors = match result { Err(CompileErrors(es)) => es, Ok(_) => Vec::new() }; assert!(!errors.is_empty() \|\| is_ok);` — **NEW TAUTOLOGY** introduced by wave-7/8. The assertion accepts EITHER "happy path" (`is_ok == true`) OR "error path with at least one error" (`!errors.is_empty() == true`). The only failure case is `Err(CompileErrors(vec![]))` (empty errors list, not Ok). This is a logical weak-OR equivalent to `is_ok() || is_err()` — passes for any production outcome. | Delete the version validation in production (return `Ok(_)` for any version). Test passes (errors is empty, is_ok is true). Delete the empty-error fallback (return `Err(CompileErrors(vec![]))` for invalid versions). Test passes (errors is empty so `!errors.is_empty()` is false; but `is_ok` is also false → assertion FAILS). Wait — this case is actually caught. But "any Ok result" silently passes — and the test name is `compile_error_unknown_version_rejected`. | Replace with concrete contract: `assert!(matches!(result, Err(CompileErrors(ref errors)) if errors.iter().any(|e| matches!(e, CompileError::InvalidVersion { .. }))), "version validation must reject 'invalid-version' string, got {result:?}");` |
| M-11 | MEDIUM | `crates/workspace_tests/tests/runtime_version_barrier_tests.rs:314,340,366,415,436,457,478,499,520,547,612,668` (12 sites — NEW wave-7/8 file, not in round 2 census) | `assert!(result.is_err());` without specific variant checks. 12 sites in this wave-7/8 test file. The file name suggests gate-count and digest-mismatch rejection tests — but `is_err()` accepts any Err. | Make `VersionGate::validate` return `Err(VersionGateError::Other)` for any input. All 12 tests pass; the prop_assert tests on lines 716,730,745 (which DO check specific fields) would still pass for known-good inputs. | Each test should `assert!(matches!(result, Err(VersionGateError::GateCountExceeded { found: c }) if c > 15))` etc., matching the specific contract. |
| L-01 | LOW | `crates/vb_storage/src/tests/chunk_012.rs:181` (NEW FINDING — test name stale) | Test name `declared_keyspaces_returns_ten_entries` says "ten" but the assertion on line 186 is `assert_eq!(keyspaces.len(), 11)`. The comment on line 184 says "exactly 11 keyspace names" — comment and assertion agree, but the test name is stale (would mislead readers about what is asserted). | n/a — purely cosmetic / misleading. | Rename to `declared_keyspaces_returns_eleven_entries`. |
| L-02 | LOW | `crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:981,993,1005` (3 sites — NEW wave-7/8 file) | `assert!(result.is_err(), "empty string must fail to parse")`, `assert!(result.is_err(), "negative string must fail to parse")`, `assert!(result.is_err(), "overflow string must fail to parse")` — accept any Err variant for `u64::from_str`-style parse failures. No variant check. | Make `parse_u64_field` return `Err(ParseError::Other)` for all invalid inputs. All 3 tests pass. | Each test should match its specific contract: `assert!(matches!(result, Err(ParseError::EmptyInput)));` etc. |
| L-03 | LOW | `crates/vb_storage/src/tests/chunk_004.rs:166,274`, `chunk_008.rs:173,181,185`, `chunk_040.rs:212,227,268` (8 sites — round-1 H-05 re-classified) | `assert!(journal.is_ok(), "...")` smoke tests. Paired with concrete round-trip assertions and explicit `declared_keyspaces().len() == 11` checks. Round-1 H-05 still not fixed but re-classified LOW because each site is followed by a concrete write-read roundtrip that catches production mutations. | Make `FjallJournal::open` return `Ok(FjallJournal::default())` without creating keyspaces. The `is_ok()` passes; but the `declared_keyspaces().len() == 11` assertion on line 175/188 would still fail (because the static function returns 11 regardless). | Convert to `assert_eq!(journal.map(\|j\| j.is_usable()), Ok(true))` — but this is mostly cosmetic. Acceptable as decorative + concrete. |
| L-04 | LOW | `crates/workspace_tests/tests/doctor_storage_scan_decode_tests.rs:1363-1364,1457-1460` (4 sites — round-2 M-06 + new) | `assert!(header_result.is_ok()); assert!(full_result.is_ok());` at lines 1363-1364 (decorative, followed by `?` on lines 1367-1368); `assert!(decoded_header.is_ok(), ...)` at line 1457-1460 (decorative, no concrete check follows). The decorative pattern is acceptable when followed by a `?` (the `?` is the real check), but line 1457 stands alone. | For line 1457: make `decode_record_header` always return `Ok(RecordHeader::default())`. Test passes; no downstream assertion catches it. | For line 1457: add `assert_eq!(decoded_header.unwrap().sequence, 1)` or similar concrete check. |
| L-05 | LOW | `crates/workspace_tests/tests/vb_test_core_workflow_slot_behavior.rs:694,730,810,997,1104,1117` (6 sites — round-1 M-02 carried over) | `assert!(result.is_err());` without specific variant checks. Round-1 M-02 partially addressed (some sites fixed), 6 remain. | Make `slot_validate` return `Err(SlotError::Other)` for any input. All 6 tests pass; the concrete variant tests elsewhere in the file catch specific errors. | Each test should match its specific contract variant. |
| L-06 | LOW | `crates/vb_storage/src/security_tests.rs:1059-1062` (`lock_releases_on_journal_drop`) | `assert!(result.is_ok(), "re-open after drop must succeed because lock was released")` — happy path for lock release. Pairs with `process_lock_file_is_created` (line 181-192) which uses `lock_path.exists()` (good). Round-1 M-11 still not addressed. | If `FjallJournal::open` always returned `Ok`, this test passes; the `process_lock_file_is_created` test catches the "no lock file" mutation. But the combined "acquire + release" lifecycle is split across 2 tests. | Add `let lock_path = temp.path().join(".process.lock"); assert!(!lock_path.exists(), ".process.lock must be released on drop");` BEFORE the second `open` (mirroring `process_lock_tests.rs:168-171`). |
| O-01 | OBSERVATION | `vb_qi37_25_quality_gates.rs:250`, `vb_c1s0_orchestration_runtime_tests.rs:593`, `vb_test_runtime_lifecycle_state_behavior.rs:469`, `vb_vt2f_direct_runtime_api_acceptance.rs:605`, `vb_njju_mutation_fuzz_property_closure.rs:224` (5 sites, same as round-1 H-09 / round-2 O-01) | `#[ignore]` on behavior tests, all documented as "pre-existing issue" or "pending GAP". No progress. | Mark them all as `#[ignore]` permanently — tests pass by not running. Production regressions in those contract areas would not be caught. | Open beads for each ignored test to track closure. The closure plan (fix or remove) was not executed across 3 review rounds. |
| O-02 | OBSERVATION | `crates/workspace_tests/tests/vb_test_cli_storage_io_behavior.rs:356,401,439,478,542,591,624,671,706,758,796,822,846,877,878,1030,1118` (17 sites) | `assert!(result.is_ok(), "events_for_run must succeed")` followed by `let events = result.expect("events ok")` and concrete `matches!(terminal, Some(JournalEvent::RunFinished { .. }))` checks. Decorative + concrete idiom (gold-standard pattern per round-1 O-03). | n/a — the concrete check makes the test meaningful. | Acceptable as decorative + concrete. No change recommended. |
| O-03 | OBSERVATION | `crates/workspace_tests/benches/action_dispatch.rs:378` (round-1 O-01 carried over) | `assert!(result.is_err() \|\| result.is_ok(), "dispatch of unknown action")` — explicit tautology in a benchmark (lower severity than round-1 C-01 because it's in a benchmark, not a test). | n/a — benchmarks are not behavior tests. | Acceptable as no-op benchmark correctness assertion. |

---

## Pattern Census

### `assert!(...is_ok()) / assert!(...is_err())` (BANNED in behavior assertions)

| Crate | Total matches | Top files (round 3) |
|-------|---------------|---------------------|
| `vb_storage/src` | 25 (down from round-2's 28 — H-02/H-03 fixed) | `hydrate_tests.rs` (3 — round-2 was 9, 6 fixed to `assert_eq!`), `codec/tests.rs` (3 — round-2 was 4, but the one at line 565 changed to `assert_eq!` while 3 sites at 2813/2825 are now `assert_eq!` too), `security_tests.rs` (2 — 767 fixed to `assert_eq!`, 1072 still smoke), `tests/chunk_040.rs` (3), `tests/chunk_008.rs` (3), `tests/chunk_004.rs` (2), `process_lock_tests.rs` (3 — 217, 225 still smoke; 175 is decorative + `?`), `recovery/recovery_unit_tests.rs` (1 — line 800 decorative), `recovery/replay/summary/tests.rs` (1), `type_tests.rs` (1), `codec/tests/replay_integrity.rs` (1 — decorative + concrete) |
| `vb_storage/tests` | 0 | (none in `crates/vb_storage/tests/`) |
| `workspace_tests/tests` | ~135 (up from round-2's 113 — wave-7/8 files added) | `integration_validate_yaml_parsing.rs` (19 — mostly bare `is_err()`), `vb_test_cli_storage_io_behavior.rs` (17 — decorative + concrete), `runtime_version_barrier_tests.rs` (15 — wave-7/8 file, mostly bare `is_err()`), `contracts_production_binding.rs` (21 — up from 15), `vb_test_core_workflow_slot_behavior.rs` (15 — partial decorative + concrete), `vb_eepg_bdd_tests.rs` (5 — wave-7/8 file), `integration_compile_error_message_quality.rs` (3 — one is the NEW tautology at 488), `doctor_storage_scan_decode_tests.rs` (13 — several decorative + concrete), `vb_8mdp_7_resource_admission_props.rs` (10, prop_assert), `vb_test_compile_parse_validate_behavior.rs` (9), `integration_validate_policy_enforcement.rs` (5), `integration_storage_runtime_validate_pipeline.rs` (4), `vb_qi37_2_4_integration_budget_errors.rs` (5), `integration_boundary_inventory_evidence_validation.rs` (2), `bdd_validation_tests.rs` (0 — gold standard) |
| `workspace_tests/benches` | 12 | Same as round 2. |
| **TOTAL** | **~172** | Slight increase from round-2's ~153, mostly due to wave-7/8 additions in `runtime_version_barrier_tests.rs` and `vb_eepg_bdd_tests.rs`. Net effect: 3 sites in vb_storage were properly tightened (H-02/H-03), but wave-7/8 added ~19 new sites. |

### Tautology assertions (`is_ok() || is_err()` or weak-OR)

| File:Line | Status |
|-----------|--------|
| All 4 round-1 tautology sites | **DELETED** (round-1 fix verified) |
| `crates/workspace_tests/benches/action_dispatch.rs:378` | **STILL PRESENT** — benchmark context, observation-level. |
| `crates/vb_storage/src/blob_tests.rs:260` | **STILL PRESENT** — `is_err() \|\| map_or(false, \|opt\| opt.is_none())` weak-OR. Re-classified MEDIUM (M-06). |
| `crates/workspace_tests/tests/integration_compile_error_message_quality.rs:488` | **NEW FINDING** — `assert!(!errors.is_empty() \|\| is_ok)` is a NEW tautology (M-10). |
| **TOTAL** | **1 hard tautology (NEW) + 1 weak-OR + 1 benchmark tautology + 1 round-1 carried weak-OR** |

### `let _ = ...` (silent error suppression)

| Crate | Total | Notes |
|-------|-------|-------|
| `vb_storage/src` | 8 | `kani_*` (3 — proof harnesses), `index_tests.rs` (7 — `let _ = item.key()` fixture iteration), `tests/chunk_*.rs` (4 — `builder.try_push()` discarding capacity check), `tests/chunk_002.rs` and `chunk_032.rs` etc. (~10 — same fixture pattern), `proptest_storage.rs` (~5 — proptest input verification), `recovery/recovery_unit_tests.rs` (1 — `_exhaustive_match`). All acceptable as fixture/proof-harness patterns. |
| `workspace_tests/tests` | ~95 | `bdd_validation_tests.rs:1364-1402` (37 — `let _ = ValidationError::VariantName` variant-existence check), `timer_deadline_primitive_tests.rs` (~50 — `wheel.insert()` discarding setup result), `bdd_idempotency.rs` (2 — `tracker.mark_completed(&ticket)` setup), `cancel_kill_lattice_tests.rs` (2 — `complete_action_with_output` / `tick_all` setup). |
| **TOTAL** | **~103** | Up ~17 from round-2's ~86. Most are fixture construction or variant-existence checks (acceptable). |

### `#[ignore]` / `#[should_panic]` / `sleep(` / `todo!()` / `unimplemented!()`

| Crate | Total | Notes |
|-------|-------|-------|
| `workspace_tests/tests` | 5 `#[ignore]` | **SAME as rounds 1+2** — no progress. All 5 documented as "pre-existing issue" or "pending GAP". See O-01. |
| `workspace_tests/tests` | 1 `sleep()` | `vb_a0t1_source_length_gate/support.rs:269` — bounded 50ms in 60s polling loop. Acceptable. |
| `workspace_tests/tests` | 0 `#[should_panic]` | Clean. |
| **TOTAL** | 6 | Unchanged from rounds 1+2. |

### `lazy_static` / `OnceLock` / `static mut` / `thread_local!`

| Crate | Total | Notes |
|-------|-------|-------|
| All test files | **0** | Clean (production has them in `vb_storage/src/journal/core.rs:125`, `vb_storage/src/queue/writer.rs:53`, `vb_storage/src/queue/loom_vb_mrwe_7.rs:8`; tests do not). |

### `panic!()` in test bodies (banned by rubric, but context-dependent)

| Crate | Total matches | Context |
|-------|---------------|---------|
| `workspace_tests/tests` | 68 | Same as round 2 — `match other => panic!("expected X, got {other:?}")` positive assertions. Acceptable. |
| `vb_storage/src` | ~70 | Same as round 2 — fixture or positive-assertion idioms. Acceptable. |
| **TOTAL** | ~138 | Unchanged. |

### `.ok();` / `.err();` silent Result conversion

| Crate | Total | Notes |
|-------|-------|-------|
| `workspace_tests/tests` | 11 | `vb_8mdp_7_resource_admission_props.rs` (11 sites) — silent discard of `enqueue`/`tick` errors. Round-2 L-02 carried over. |
| `vb_storage/src/proptests.rs` | 2 | Round-2 L-03 carried over. |
| **TOTAL** | 13 | Unchanged. |

### Fuzz/kani mis-categorized as `#[test]`

| File | Status |
|------|--------|
| `crates/vb_storage/src/kani_*.rs` (~80 files) | All `#[cfg(all(kani, feature = "..."))]` gated. CLEAN. |
| `crates/fuzz/fuzz_targets/*.rs` | Outside slice 2 scope. CLEAN. |
| **TOTAL fuzz/kani mis-categorized** | **0** |

---

## Mutation Gaps (top 5 most dangerous bugs the slice would NOT catch)

1. **`compile_error_message_quality.rs:488` tautology accepts both happy and error paths.**
   The test asserts `assert!(!errors.is_empty() || is_ok)` — passes if production returns Ok (line 482 `is_ok = true`) OR returns `Err(CompileErrors(vec![any]))` (line 488 `!errors.is_empty() = true`). A mutation that makes `parse_ast` accept any version string and return `Ok(_)` would pass. **The only failure case is `Err(CompileErrors(vec![]))` (empty error list, not Ok)**, but that is an unusual production mutation. **File:Line:** production `crates/vb_compile/src/yaml_compiler.rs::parse_ast`.

2. **`validate_tail_first_seq_contiguous_with_snapshot` accepts any input (delete the contiguity check).**
   The 3 `assert!(result.is_ok())` sites in `hydrate_tests.rs:281,287,374` would all pass. The 4 error-path tests on adjacent lines (`validate_tail_first_seq_contiguous_rejects_gap`, `_rejects_equal_to_snapshot`, `validate_tail_events_after_snapshot_rejects_non_contiguous_first`) still fail for specific inputs, so partial mutation caught. **File:Line:** `crates/vb_storage/src/recovery/hydrate/validation.rs:88`.

3. **`open_store` / `init_keyspaces` silently skip `acquire_process_lock`.**
   `open_store_acquires_process_lock` (process_lock_tests.rs:213-218) and `init_keyspaces_acquires_process_lock` (220-228) only check `is_ok()`. The `process_lock_file_is_created` test (line 181-192) catches the lock-file absence via `lock_path.exists()` — but a mutation where `open_store` calls a different code path that creates the lock file WITHOUT going through `acquire_process_lock` would silently pass. **File:Line:** `crates/vb_storage/src/lib.rs::open_store` and `crates/vb_storage/src/lib.rs::init_keyspaces`.

4. **`journal.blob(digest)` returns `Ok(None)` for corrupt data.**
   `blob_tests.rs:260` accepts EITHER `Err(_) | Ok(None)`. A mutation that silently downgrades "corrupt blob" to "blob not present" would pass the test, hiding a SECURITY-relevant failure mode. Round-2 M-07 still unfixed. **File:Line:** `crates/vb_storage/src/journal/blob_reader.rs::blob`.

5. **`process_lock_tests.rs:213-228` smoke tests hide `acquire_process_lock` no-op.**
   Same as gap 3 but emphasized: 2 SECURITY-relevant tests still smoke. Round-1 H-04 has been carried over 3 rounds without fix. **File:Line:** `crates/vb_storage/src/process_lock.rs::acquire`.

---

## Top 5 Fixes (impact-per-effort)

### Fix 1 — Convert 3 remaining `assert!(result.is_ok())` in `hydrate_tests.rs:281,287,374` to `assert_eq!`
**Impact:** 3 success-path tests gain concrete value assertions. Functions return `Result<(), _>` so direct conversion. **Effort:** 5 minutes.

```rust
// crates/vb_storage/src/hydrate_tests.rs:281
// BEFORE:
assert!(result.is_ok(), "seq 6 must be contiguous with snapshot seq 5, got {:?}", result);
// AFTER:
assert_eq!(result, Ok(()), "seq 6 must be contiguous with snapshot seq 5");
```

### Fix 2 — Convert 2 `assert!(result.is_ok())` in `process_lock_tests.rs:213-228` to assert lock file presence
**Impact:** 2 SECURITY-relevant tests now verify the full "open_store acquires lock" contract. **Effort:** 10 minutes.

```rust
// crates/vb_storage/src/process_lock_tests.rs:213-218
// BEFORE:
#[test]
fn open_store_acquires_process_lock() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let result = crate::open_store(temp.path());
    assert!(result.is_ok(), "open_store should acquire process lock");
}
// AFTER:
#[test]
fn open_store_acquires_process_lock() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let result = crate::open_store(temp.path());
    assert!(result.is_ok(), "open_store must succeed, got {result:?}");
    let lock_path = temp.path().join(".process.lock");
    assert!(lock_path.exists(), "open_store must create .process.lock");
}
```

### Fix 3 — Delete the NEW tautology at `integration_compile_error_message_quality.rs:488`
**Impact:** 1 test becomes a real contract test instead of a wave-7/8 regression. **Effort:** 5 minutes.

```rust
// BEFORE (line 479-488):
let result = YamlCompiler::default().parse_ast(source);
let is_ok = result.is_ok();
let errors = match result {
    Err(CompileErrors(es)) => es,
    Ok(_) => Vec::new(),
};
assert!(!errors.is_empty() || is_ok);
// AFTER:
let result = YamlCompiler::default().parse_ast(source);
assert!(
    matches!(result, Err(CompileErrors(ref errors)) if errors.iter().any(|e| matches!(e, CompileError::InvalidVersion { .. }))),
    "version 'invalid-version' must be rejected with InvalidVersion, got {result:?}"
);
```

### Fix 4 — Rename `chunk_012.rs:181` from `declared_keyspaces_returns_ten_entries` to `declared_keyspaces_returns_eleven_entries`
**Impact:** Eliminates misleading test name. **Effort:** 1 minute.

### Fix 5 — Triage the 5 `#[ignore]` tests + the 21 `is_err()` sites in `contracts_production_binding.rs`
**Impact:** 26 sites across 2 file categories get concrete variant checks OR are removed. **Effort:** Variable; the `contracts_production_binding.rs` is ~30 minutes (21 sites), the `#[ignore]` triage depends on contract fixability.

---

## Round-3 Summary

| Category | Count |
|----------|-------|
| Round-1 CRITICAL fixes verified STILL APPLIED | 4 / 4 |
| Round-2 CRITICAL fixes verified STILL APPLIED | 3 / 3 (count + corruption tests) |
| Round-1 HIGH carry-overs addressed in round 3 | 3 of 10 (H-01 mostly, H-02 fully, H-03 fully) |
| **CRITICAL findings (NEW)** | **0** |
| HIGH findings (NEW) | 0 |
| MEDIUM findings (NEW + carry-over) | 11 (M-01 through M-11) |
| LOW findings (NEW + carry-over) | 6 (L-01 through L-06) |
| OBSERVATION findings (NEW + carry-over) | 3 (O-01 through O-03) |
| Round-1 regression count | 0 |
| Round-2 regression count | 0 |
| vb_storage --tests pass count | **1760 passed, 0 failed** (matches round-2 baseline) |

The slice is **substantially improved** vs. round 1+2. Round-1's 3 CRITICAL + 2 round-2 CRITICAL fixes all held with zero regressions. The 3 most impactful HIGH carry-overs (H-01 hydrate, H-02 codec, H-03 security digest) were substantially tightened — most sites now use `assert_eq!(result, Ok(()), ...)` with security-relevant comments. Wave-7/8 introduced 1 NEW MEDIUM tautology (`integration_compile_error_message_quality.rs:488`) and 1 NEW LOW stale test name (`chunk_012.rs:181`) — both are isolated, easy fixes. The remaining defects are concentrated in pre-wave-5 test files that were not touched in the round-1 fix cycle.

The slice's strengths are notable: BDD scenario file (`bdd_validation_tests.rs`) remains exemplary, `journal_event_tests.rs` decoder tests are the gold standard, `bdd_idempotency.rs` uses precise `assert_eq!` patterns, and the new `vb_2bok_durability_gate_tests.rs` (round-2 O-05) demonstrates zero banned patterns.

---

## Disposition

| ID | Disposition | Rationale |
|----|-------------|-----------|
| M-01 through M-11 | `blocker` | Round-1 HIGH carry-overs (H-01, H-04, H-06, H-07, H-10) + round-2 M-04..M-07 not addressed; plus 1 NEW MEDIUM tautology (M-10) and 1 NEW MEDIUM wave-7/8 file (M-11). Together ~30 sites that let mutations in `validate_tail_first_seq_contiguous_with_snapshot`, `process_lock::acquire`, `parse_schema_version`, `parse_vet_exit_code`, `journal.blob`, `JournalBatchSize::try_from_usize`, `YamlCompiler::parse_ast`, `VersionGate::validate` pass silently. **REJECTED.** |
| L-01 through L-06 | `owner_approved_debt` | Track as round-4 candidates; each is a tractable refactor but not a regression blocker. L-03 (chunk_* smoke tests) is decorative + concrete. L-04 (decorative + concrete pattern in scan_decode) is the documented gold-standard idiom. L-06 (security_tests lock-release) is paired with a separate concrete check. |
| O-01 through O-03 | `owner_approved_no_action` | `#[ignore]` tests are documented (O-01); decorative + concrete is the documented gold-standard idiom (O-02); benchmark tautology is out of behavior-test scope (O-03). |

---

## Verdict

```
STATUS: REJECTED
```

**0 NEW CRITICAL + 11 NEW/CARRY-OVER MEDIUM findings**, with **0 round-1+2 regressions** (all 4 round-1
CRITICAL + 2 round-2 CRITICAL fixes held) but **7 round-1 HIGH carry-overs** still unfixed (H-04, H-05,
H-06, H-07, H-09, H-10 — and 3 sites in H-01 missed). The slice cannot advance until:

1. **M-01** — convert 3 `assert!(result.is_ok())` in `hydrate_tests.rs:281,287,374` to `assert_eq!` (5 min).
2. **M-02** — add `lock_path.exists()` checks to `process_lock_tests.rs:213-228` (10 min).
3. **M-10** — delete the NEW tautology at `integration_compile_error_message_quality.rs:488` (5 min).
4. **M-03, M-04, M-05** — tighten 21 `is_err()` sites in `contracts_production_binding.rs` + 1 in `vb_test_compile_parse_validate_behavior.rs:184-188` (~30 min).
5. **M-06, M-07, M-08, M-09, M-11** — tighten weak-OR + variant checks across 5 files (~30 min).

Total cleanup time: ~80 minutes plus triage of the 5 `#[ignore]` tests and the 21 contracts_production_binding sites.

Round-3 progress vs. round 2:
- Round 1: 3 CRITICAL + 10 HIGH + 12 MEDIUM + 8 LOW + 5 OBSERVATION
- Round 2: 2 CRITICAL (new) + 0 HIGH + 6 MEDIUM (new) + 4 LOW (new) + 5 OBSERVATION
- Round 3: 0 CRITICAL + 0 HIGH + 11 MEDIUM + 6 LOW + 3 OBSERVATION

The CRITICAL and HIGH counts are now zero (down from 3+10 in round 1, 2+0 in round 2). MEDIUM count
increased slightly (11 vs. 6) due to wave-7/8 file additions and carry-overs not yet addressed.
LOW count is consistent (6 vs. 8 in round 1, 4 in round 2). The convergence pattern matches the
expected round-3 trajectory: critical defects are eliminated, remaining work is MEDIUM/LOW cleanup.