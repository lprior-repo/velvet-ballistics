# Test Review — Slice 2: vb_storage + workspace_tests

**Scope:** 313 test-bearing Rust files (116 in `crates/vb_storage/`; 197 in `crates/workspace_tests/`; plus benches).
The "318" cited in the brief over-counted by file count vs. `#[test]`-bearing file count. Kani harnesses
(`kani_*.rs`, ~80 files) are correctly `#[cfg(kani)]`-gated proof artifacts and are excluded from
behavior-test review.

**Date:** 2026-06-21
**Reviewer:** test-reviewer agent (slice 2 of 4)

## STATUS: REJECTED

The slice contains one CRITICAL tautology assertion that proves nothing (4 sites across 2 files),
two CRITICAL `match-all-outcomes` tests in `process_lock_tests.rs` that always pass regardless of
behavior, and one CRITICAL test whose name and assertion directly contradict (the "rejects" test
that only asserts "accepts"). The slice also has 6 HIGH-severity `assert!(...is_ok())` smoke tests
without concrete value verification, two of which are for security-relevant digest-match and
process-lock contracts. Recovery and codec tests are otherwise strong — most use concrete
`assert_eq!` and `matches!(result, Err(SpecificVariant{..}))` patterns. The BDD-style
`bdd_validation_tests.rs` (62 scenarios) and `bdd_runner_tests.rs` are exemplary. The slice has
**3 CRITICAL**, **10 HIGH**, **12 MEDIUM**, **8 LOW**, and **5 OBSERVATION** findings; cannot
be approved.

---

## Findings (CRITICAL first)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix |
|----|-----|-----------|--------|------------------------------|------------------|
| C-01 | CRITICAL | `crates/workspace_tests/tests/integration_compile_error_message_quality.rs:376,401,424` | `assert!(result.is_ok() \|\| result.is_err())` — a logical tautology that holds for ANY Result. The test passes if production panics (return None), always returns Ok, always returns Err, or returns complete garbage. | Delete `YamlCompiler::compile` entirely; replace with `fn compile(...) -> Result<CompiledWorkflow, CompileError> { Ok(CompiledWorkflow::default()) }`. All three "CompileError::DepthLimit/SequenceLimit/ScalarLimit" tests pass. | Replace each tautology with `assert!(result.is_err(), "limit must reject input: {result:?}")` and assert the specific variant: `assert!(matches!(result, Err(CompileError::DepthLimit { depth: d }) if d == 1));` |
| C-02 | CRITICAL | `crates/workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:215` | `assert!(result.is_ok() \|\| result.is_err()); // boundary is permissive on empty seed` — explicit tautology with a self-justifying comment. The test name ("hydration of empty seed") implies a contract; the test asserts no contract. | Delete `hydrate_run_frame`'s seed validation entirely; return `Err(RecoveryError::UnsupportedState)` always. Test still passes. | Pick the contract: assert `matches!(result, Ok(RecoveredRunFrame { steps: vec![], ... }))` for a known-empty seed. |
| C-03 | CRITICAL | `crates/vb_storage/src/process_lock_tests.rs:141-181` | `process_lock_prevents_dual_writers_same_directory` (lines 141-162) and `process_lock_is_released_on_drop` (lines 164-181) use `match result { Ok(_) => {} \| Err(ProcessLockHeld) => {} \| Err(other) => { _ = other; } }` — the test accepts ANY outcome. The `_ = other;` on line 159 actively suppresses the error. Test name claims a contract; body proves none. | Delete `process_lock.rs` entirely. Both tests pass because every arm is `()`. | Replace each `_ =>` arm with `panic!("unexpected outcome: {result:?}")`. For lock-rejection: `assert!(matches!(result, Err(JournalError::ProcessLockHeld { .. })));`. For lock-release: assert the second open `Ok`s, then mutate state and verify. |
| C-04 | CRITICAL | `crates/vb_storage/src/edge_case_tests.rs:547-554` | Test name `encode_rejects_zero_length_payload_serialization` asserts the OPPOSITE: `assert!(result.is_ok(), "empty payload should be accepted")`. The test name lies about what it tests. The body asserts that empty payloads are ACCEPTED, so deleting the entire "reject" branch in production would not change the test outcome. | In `codec::encode_record`, delete the `if bytes.is_empty() { return Err(...) }` check. Test passes. | Rename test to `encode_accepts_zero_length_payload`. Or, if rejection is intended, `assert!(matches!(result, Err(JournalError::PayloadEmpty)));` |
| H-01 | HIGH | `crates/vb_storage/src/hydrate_tests.rs:132,155,178,208,214,220` (6 sites) | `assert!(result.is_ok(), "matching run should succeed, got {:?}", result)` — smoke tests on success path with no concrete value verification. The corresponding failure tests (lines 136-148, 159-170, etc.) use `matches!(result, Err(SnapshotRecoveryInputViolation::SpecificVariant{..}))` — so error variants are caught. But success paths would silently allow production to return `Ok(())` always (delete the check). | In `validate_snapshot_metadata`, replace the check with `Ok(())`. The 6 success tests pass. The 4 error tests still fail — so partial mutation (delete success-path check) is not caught. | Replace with `assert_eq!(result, Ok(()));` (explicit unit) or assert the returned `ValidatedSnapshot` carries the expected `run` and `seq` fields. |
| H-02 | HIGH | `crates/vb_storage/src/codec/tests.rs:565,2760,2772` (3 sites) | `assert!(result.is_ok(), "correct digest should pass verification")` (line 565), `assert!(result.is_ok(), "zero run and seq should pass validation")` (line 2760), `assert!(result.is_ok(), "max run and seq should pass validation")` (line 2772). Boundary tests for `verify_digest_match` and `validate_replayed_event` — return value carries no checked field. | Make `verify_digest_match` always return `Ok(())`. Test on line 565 passes; the wrong-digest test on line 569-578 still fails, so partial mutation is caught. Make `validate_replayed_event` always return `Ok(ValidatedEvent::default())` — the zero/max tests pass but no test verifies the returned event fields. | For line 565: replace with `assert_eq!(result, Ok(()))`. For lines 2760, 2772: assert `assert_eq!(result.unwrap().run, RunId::new(0));` and `.seq == EventSeq::new(0);` (or u64::MAX). |
| H-03 | HIGH | `crates/vb_storage/src/security_tests.rs:767` | `assert!(result.is_ok(), "correct digest should pass verification")` — security-relevant digest verification test. If `verify_digest_match` returns `Ok(())` for ANY payload, the security contract is silently broken. | Modify `verify_digest_match` to `Ok(())` always. The wrong-digest test on line 776 still fails (good), but a partial mutation that ALWAYS returns Ok for correct digest AND silently returns Ok for wrong digest with a different code path is not caught. | Replace with `assert_eq!(result, Ok(()))` and assert the specific returned value carries the expected digest bytes. |
| H-04 | HIGH | `crates/vb_storage/src/process_lock_tests.rs:215-230` | `open_store_acquires_process_lock` (line 215-220) and `init_keyspaces_acquires_process_lock` (line 222-230) only assert `assert!(result.is_ok())`. The function name claims "acquires process lock" but no assertion checks that any lock was actually acquired. | In `open_store`, skip the `acquire_process_lock()` call. Both tests pass. The `.process.lock` file check on line 184-194 would still catch this — but only because that test uses `lock_path.exists()` (concrete assertion), so the failure is caught elsewhere. However, `open_store` and `init_keyspaces` themselves are not tested for the lock contract directly. | After the `assert!(result.is_ok())`, assert `lock_path.exists()` (copy from line 184-194 pattern). |
| H-05 | HIGH | `crates/vb_storage/src/tests/chunk_008.rs:173,181,185` and `chunk_004.rs:166,270` and `chunk_040.rs:212,227,268` (8 sites) | `assert!(journal.is_ok(), "...")` for `FjallJournal::open` and `open_store` — does not check that any keyspaces are created or that the journal is usable. The follow-up `assert_eq!(FjallJournal::declared_keyspaces().len(), 10)` on line 186 catches one mutation (no keyspaces), but not all. | Modify `open_store` to return `Ok(FjallJournal::default())` without creating keyspaces; tests on lines 181-186 pass for the second open (which finds the existing keyspaces). | After `assert!(journal.is_ok())`, perform a write-read roundtrip: `journal.append_journaled(&event)?; journal.events_for_run(run)?;` |
| H-06 | HIGH | `crates/workspace_tests/tests/contracts_production_binding.rs:170-175,218-222,270-273,282,287-290,659-661` (15 sites) | `assert!(parse_schema_version("").is_err())` and `assert!(ContractKind::parse("bogus").is_err())` — `is_err()` accepts any Err variant. The contract requires specific MISSING_SCHEMA_VERSION and INVALID_VERSION error codes; a mutation that returned `Err(ContractError::Unknown)` would silently pass. | Change `parse_schema_version` to return `Err(ContractError::Unknown(""))` for all invalid inputs. The 6 invalid-input tests on lines 170-175 pass; the test on line 180-181 (`err.to_string() == "MISSING_SCHEMA_VERSION"`) catches this — but only for the empty-string case. For "abc", "v1.0.0", etc., no error-code check exists. | Replace each `assert!(...is_err())` with `let err = ....unwrap_err(); assert!(err.to_string().contains("MISSING_SCHEMA_VERSION") \|\| err.to_string().contains("INVALID_VERSION"));` |
| H-07 | HIGH | `crates/workspace_tests/tests/vb_test_compile_parse_validate_behavior.rs:184-188` | `parse_rejects_whitespace_only_source` uses `assert!(result.is_err(), "whitespace-only source should fail")` — accepts any error. The previous test `parse_rejects_totally_empty_source` (line 172-182) checks the message contains "empty" or "document" — but the whitespace-only test does NOT check the message. | Make `YamlCompiler::parse_ast(b"   \n")` return `Err(CompileError::Other)`. Test passes; the test for empty source would still fail. | Add: `let msg = result.unwrap_err().to_string(); assert!(msg.contains("empty") \|\| msg.contains("whitespace"));` |
| H-08 | HIGH | `crates/workspace_tests/tests/integration_compile_error_message_quality.rs:376,401,424` (3 sites — overlap with C-01) | Same tautology as C-01. Listed separately as HIGH because it represents 3 distinct test functions (`compile_error_depth_limit_includes_depth_value`, `compile_error_sequence_limit_exists_and_configurable`, `compile_error_scalar_limit_exists`). | Same as C-01. | Same as C-01. |
| H-09 | HIGH | `crates/workspace_tests/tests/vb_qi37_25_quality_gates.rs:249-250`, `vb_c1s0_orchestration_runtime_tests.rs:592-593`, `vb_test_runtime_lifecycle_state_behavior.rs:468-469`, `vb_vt2f_direct_runtime_api_acceptance.rs:604-605`, `vb_njju_mutation_fuzz_property_closure.rs:223-224` (5 sites) | `#[ignore]` on behavior tests. Each test has a documented comment explaining why it's ignored (pre-existing failure, pending gap, etc.). These tests claim a contract that the codebase cannot currently satisfy. | Mark them all as `#[ignore]` permanently — tests pass by virtue of not running. Production regressions in those contract areas would not be caught. | Either fix the contract or remove the tests entirely. Tracking ignored tests as "debt" without a closure date is a maintenance hazard. |
| H-10 | HIGH | `crates/workspace_tests/tests/contracts_production_binding.rs:282` | `assert!(parse_vet_exit_code(0).is_ok())` — `parse_vet_exit_code(0)` is the success path; the only assertion is `is_ok()`. The function returns `Result<i32, String>` — but the test does not check that the returned `i32` is `0` (the actual contract value). | Make `parse_vet_exit_code` always return `Ok(42)`. Test passes. | Replace with `assert_eq!(parse_vet_exit_code(0), Ok(0));` |
| M-01 | MEDIUM | `crates/workspace_tests/tests/integration_validate_policy_enforcement.rs:274,462` (and ~6 more sites) | `assert!(result.is_ok(), "single slot load should pass: {:?}", result)` and `assert!(result.is_ok(), "linear workflow should pass: {:?}", result)` — happy-path tests for `validate(&parts)`. Concrete enough that no production mutation is currently missed, but very fragile to refactor. | Could replace `validate` with `Ok(())` and pass all 8+ tests; but the error-path tests would catch this. Partial mutation not caught. | Convert to `assert_eq!(result, Ok(()))` for explicit unit comparison. |
| M-02 | MEDIUM | `crates/workspace_tests/tests/integration_validate_yaml_parsing.rs:174,198,218,238,266,286,302,336,374,388,398,404,410` (13 sites) | `assert!(result.is_err())` without verifying the specific error variant. The contract is "rejects anchor", "rejects tag", "rejects merge key" — but the tests accept any Err. | Make `validate_yaml_profile` return `Err(YamlError::Other)` for all invalid inputs. All 13 invalid tests pass; the 6 happy-path tests still fail. | Each test should match its specific contract: `assert!(matches!(result, Err(YamlError::AnchorNotSupported)));` etc. |
| M-03 | MEDIUM | `crates/workspace_tests/tests/integration_boundary_inventory_evidence_validation.rs:83,326` | `assert!(result.is_ok(), "freshness should be valid: {:?}", result)` and `assert!(result.is_ok(), "CAbi should be valid: {:?}", result)` — happy-path tests. | If `validate_inventory` returns `Ok(())` for any input, these tests pass; but the file's other tests verify specific rejection of stale/invalid records. | Convert to `assert_eq!(result, Ok(()))`. |
| M-04 | MEDIUM | `crates/vb_storage/src/recovery/recovery_unit_tests.rs:789` | `assert!(result.is_err());` immediately followed by `assert!(matches!(result.unwrap_err(), RecoveryError::NoRecoveryData { .. }))` (lines 790-793). The first assertion is decorative — the second is the real check. | n/a — the mutation thought experiment does not apply because the second assertion is concrete. | Remove the redundant `assert!(result.is_err())` on line 789. |
| M-05 | MEDIUM | `crates/workspace_tests/tests/doctor_key_decode_tests.rs:164` | `assert!(config.is_ok())` — `PreviewConfig::new(100, 4096)` is the happy-path; the test does not verify that `max_records()` and `max_bytes()` return the expected values. Lines 178-181 verify these in a SEPARATE test, so the contract IS covered — but at a per-test granularity. | n/a — mutation would be caught by the dedicated test on line 178-181. | Either delete this test (covered by line 178) or assert `config.unwrap().max_records().get() == 100`. |
| M-06 | MEDIUM | `crates/workspace_tests/tests/doctor_storage_scan_decode_tests.rs:1363-1364` | `assert!(header_result.is_ok()); assert!(full_result.is_ok());` followed by `let header = header_result?; let (envelope, _event) = full_result?;`. The two `is_ok()` assertions are decorative — the `?` operators on lines 1367-1368 do the real work. | n/a — both decorative and the real check. | Remove lines 1363-1364 (redundant). |
| M-07 | MEDIUM | `crates/workspace_tests/tests/doctor_storage_scan_decode_tests.rs:1278,1314` | `Err(JournalError::Fjall(_)) => { /* expected */ } \| Err(_) => { /* other typed error also acceptable */ } \| Ok(_) => panic!`. Accepts any Err variant for `parse_decode_error_invalid_keyspace_path` and `parse_decode_error_corrupt_journal_bad_magic`. The contract is "must return typed error", not "must return ANY error". | Make `FjallJournal::open` return `Err(JournalError::ProcessLockIo { ... })`. Both tests pass; the test on line 1313 checks `BadMagic` field for the magic-corruption case (good), but the keyspace test has no variant check. | For line 1278: `assert!(matches!(result, Err(JournalError::Fjall(_) \| JournalError::PathNotFound { .. })));` |
| M-08 | MEDIUM | `crates/workspace_tests/tests/contracts_integration.rs:522-543` | `assert!(result.is_err(), "discover_contracts should error on nonexistent dir")` followed by `assert!(err.contains("does not exist"))`. Accepts any error, then message-contains check. | Make `discover_contracts` return `Err("some message that contains 'does not exist'")` for all errors. Test passes. | OK as-is — message check makes it concrete. Listed for completeness. |
| M-09 | MEDIUM | `crates/workspace_tests/tests/vb_qi37_2_4_integration_budget_errors.rs:2202,2284` (and ~3 more) | `assert!(policy.validate(&budget).is_ok());` — happy-path. The error-path tests on lines 2207-2210 check specific variants (good). | Replace `policy.validate` with `Ok(())`. Happy-path tests pass; error-path tests still fail. | Convert to `assert_eq!(policy.validate(&budget), Ok(()))`. |
| M-10 | MEDIUM | `crates/vb_storage/src/tests/chunk_004.rs:166,270`, `chunk_040.rs:212,227,268` | `assert!(journal.is_ok(), "...")` smoke tests without state verification. See H-05. | See H-05. | See H-05. |
| M-11 | MEDIUM | `crates/vb_storage/src/security_tests.rs:1056` | `assert!(result.is_ok(), "re-open after drop must succeed because lock was released")` — happy path for lock release. Pairs with `no_keyspace_created_when_lock_fails` on line 1062 which uses `is_err()` (concrete value check on file count after, good). | If `FjallJournal::open` always returned `Ok`, this test passes; the second-open test on line 1072 catches the partial mutation (must fail on second open). | Add: `let lock_path = temp.path().join(".process.lock"); assert!(lock_path.exists());` after `assert!(result.is_ok())`. |
| L-01 | LOW | `crates/workspace_tests/tests/timer_deadline_primitive_tests.rs:616,632,653,671,690,707,725,744,755,882,899,1052,1194,1237,1294,1330,1371,1388,1432,1444,1456,1477,1500,1511,1643,1692,1707,2030` (28 sites) | `panic!("...")` in `match other => panic!(...)` arms. Listed as banned pattern but in this context it's a positive assertion that the wrong variant was NOT matched. Standard Rust testing idiom. | n/a — would require deleting the exhaustiveness check. | Document in test module header: `// panic! in `other` arms is a positive assertion that the correct variant was matched.` Acceptable as `owner_approved_no_action`. |
| L-02 | LOW | `crates/workspace_tests/tests/postcard_envelope_wire_tests.rs:545,575,602,684,713,738` (6 sites) | `prop_assert!(result.is_err(), "...")` and `prop_assert!(result.is_ok(), "valid record should decode successfully")` (line 738). Error-path assertions don't check specific variant. | See M-02 analysis. | Convert each to `prop_assert!(matches!(result, Err(SpecificVariant)));`. |
| L-03 | LOW | `crates/vb_storage/src/keys/decode.rs:97-99` (3 sites, in doc comments) | `/// assert_eq!(try_key_prefix(&[0x01]).unwrap(), KeyPrefix::WorkflowSource); /// assert!(try_key_prefix(&[]).is_err());` — these are doc-test patterns, not real tests. | n/a — doc-tests would be compiled and run by `cargo test --doc`. | Keep — they're documentation, not behavior tests. |
| L-04 | LOW | `crates/workspace_tests/benches/velvet_ballistics.rs:933,938,948` (3 sites) | `#[test]` functions inside a `benches/` file. Tests of benchmark helper functions (`latency_within_budget`, `budget_failure_message`, `budget_success_message`). | n/a — these are helper tests, not behavior. | Acceptable — or extract to a `benches/tests.rs`. |
| L-05 | LOW | `crates/vb_storage/src/kani_*.rs` (80+ files) | Kani harnesses correctly gated by `#[cfg(all(kani, feature = "..."))]`. They are PROOF artifacts, not behavior tests. They count toward the 318-file total but are excluded from behavior-test review per the rubric. | n/a. | Out of scope. |
| L-06 | LOW | `crates/workspace_tests/tests/vb_a0t1_source_length_gate/support.rs:269` | `thread::sleep(Duration::from_millis(50))` in a 60-second timeout polling loop. Bounded, controlled, not a timing-dependent assertion. | n/a — the actual assertion is on `started.elapsed() >= Duration::from_secs(60)`. | Acceptable. |
| L-07 | LOW | `crates/vb_storage/src/security_tests.rs:767` and `codec/tests.rs:565` | `assert!(result.is_ok(), "correct digest should pass verification")` — duplicates H-03 / H-02. Listed for cross-reference. | See H-02 / H-03. | See H-02 / H-03. |
| L-08 | LOW | `crates/vb_storage/src/journal/incident/model/tests_evidence.rs` (and similar `tests_*.rs` submodules) | Module-level `#![allow(...)]` blocks (100+ lints) suppressing all clippy warnings including `unwrap_used` and `let_underscore_must_use`. Hides silent error suppression patterns. | n/a — test infrastructure. | Refactor: reduce allow list to lints actually needed; per-`#[test]` `#[allow(...)]` for the rest. |
| O-01 | OBSERVATION | `crates/vb_storage/src/preview/preview_red_queen_tests.rs:87-326` | Property-style tests using `unwrap()` heavily. Acceptable for fixture construction (raw byte arrays), but the `result.truncated`, `result.total_keyspace_records`, `result.entries.len()` assertions are concrete (good). | n/a. | Out of scope. |
| O-02 | OBSERVATION | `crates/workspace_tests/tests/bdd_validation_tests.rs:1-1553` | 62 BDD scenarios (B1-B62). Most use `assert_eq!(result, Ok(()))` for happy paths and `matches!(result, Err(ValidationError::SpecificVariant{..}))` for error paths. This is the gold standard for this slice. | n/a — exemplary. | Out of scope. |
| O-03 | OBSERVATION | `crates/vb_storage/src/journal/journal_event_tests.rs:30-642` | Uses the `match result { Err(JournalError::BadMagic { found }) => assert_eq!(found, ...), other => panic!(...) }` pattern. Concrete variant + field check + exhaustive match. This is the gold standard for this slice. | n/a — exemplary. | Out of scope. |
| O-04 | OBSERVATION | `crates/vb_storage/src/codec/tests/replay_integrity.rs:195-196` | `assert!(result.is_ok(), "next_seq(u64::MAX-1) must succeed"); assert_eq!(result.unwrap(), EventSeq::new(u64::MAX));` — the `is_ok()` is decorative (followed by concrete equality), but the concrete equality is the real check. | n/a — covered by line 196. | Out of scope. |
| O-05 | OBSERVATION | `crates/workspace_tests/tests/contracts_production_binding.rs:163-166` | `assert_eq!(parse_schema_version("1.0.0"), Ok("1.0.0".to_string()));` — concrete value check on the success path. This is the gold standard for this slice. | n/a — exemplary. | Out of scope. |

---

## Code Snippets (CRITICAL/HIGH BEFORE/AFTER)

### C-01 / H-08 — Tautology assertion in `integration_compile_error_message_quality.rs`

```rust
// BEFORE (integration_compile_error_message_quality.rs:356-377):
/// CompileError::DepthLimit: nesting depth limit enforced.
#[test]
fn compile_error_depth_limit_includes_depth_value() {
    let compiler = YamlCompiler::new(vb_compile::YamlLimits {
        max_depth: 1,
        ..Default::default()
    });
    let source = br#"
version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = compiler.compile(source);
    // Note: max_depth may not be enforced in all compile paths
    // This test verifies the limit type exists and can be configured
    assert!(result.is_ok() || result.is_err());   // <-- TAUTOLOGY
}

// AFTER:
#[test]
fn compile_error_depth_limit_includes_depth_value() {
    let compiler = YamlCompiler::new(vb_compile::YamlLimits {
        max_depth: 1,
        ..Default::default()
    });
    let source = br#"
version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = compiler.compile(source);
    assert!(
        matches!(result, Err(vb_compile::CompileError::DepthLimit { depth, max }) if depth > 1 && max == 1),
        "max_depth=1 must reject nesting > 1, got {result:?}"
    );
}
```

### C-02 — Tautology assertion in `integration_runtime_storage_fault_tolerance.rs`

```rust
// BEFORE (integration_runtime_storage_fault_tolerance.rs:210-216):
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);
    // Hydration should succeed because the seed itself is valid (corrupt snapshot
    // is a storage-layer concern; the boundary only validates the seed shape).
    let result = boundary.hydrate_run_frame();
    // A seed with step_count=0 and no workflow may still be a valid empty-run seed.
    assert!(result.is_ok() || result.is_err()); // boundary is permissive on empty seed

// AFTER:
    let boundary = DurableFrameRecoveryBoundary::from_seed(seed);
    let result = boundary.hydrate_run_frame();
    // Document the contract: empty seed must hydrate to an empty but valid frame.
    assert!(
        matches!(result, Ok(recovered) if recovered.steps.is_empty() && recovered.workflow.is_none()),
        "empty seed must hydrate to an empty run frame, got {result:?}"
    );
```

### C-03 — `match-all-outcomes` in `process_lock_tests.rs`

```rust
// BEFORE (process_lock_tests.rs:141-181):
#[test]
fn process_lock_prevents_dual_writers_same_directory() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let _journal1 = crate::FjallJournal::open(temp.path(), None)
        .expect("first journal should open successfully");

    let result = crate::FjallJournal::open(temp.path(), None);

    match result {
        Ok(_) => {
            // Fjall may allow re-opening depending on its internal behavior
            // If the second open succeeds, it means fjall handles this internally
        }
        Err(JournalError::ProcessLockHeld { .. }) => {
            // This is the expected failure mode on most systems
        }
        Err(other) => {
            // Any other error is acceptable as long as it's not a panic
            _ = other;
        }
    }
}

#[test]
fn process_lock_is_released_on_drop() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    {
        let _journal = crate::FjallJournal::open(temp.path(), None)
            .expect("first journal should open");
        // Drop happens here
    }
    // After drop, we should be able to open again
    let result = crate::FjallJournal::open(temp.path(), None);
    match result {
        Ok(_) => {}
        Err(e) => {
            // On some systems the lock may take time to release
            _ = e;
        }
    }
}

// AFTER:
#[test]
fn process_lock_prevents_dual_writers_same_directory() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let _journal1 = crate::FjallJournal::open(temp.path(), None)
        .expect("first journal should open successfully");

    let result = crate::FjallJournal::open(temp.path(), None);
    assert!(
        matches!(result, Err(JournalError::ProcessLockHeld { .. })),
        "second open must fail with ProcessLockHeld, got {result:?}"
    );
}

#[test]
fn process_lock_is_released_on_drop() {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let lock_path = temp.path().join(".process.lock");
    {
        let _journal = crate::FjallJournal::open(temp.path(), None)
            .expect("first journal should open");
        assert!(lock_path.exists(), ".process.lock must exist while journal is open");
    }
    assert!(!lock_path.exists(), ".process.lock must be released on journal drop");

    let result = crate::FjallJournal::open(temp.path(), None);
    assert!(result.is_ok(), "re-open after drop must succeed, got {result:?}");
}
```

### C-04 — Test name contradicts assertion in `edge_case_tests.rs`

```rust
// BEFORE (edge_case_tests.rs:546-554):
#[test]
fn encode_rejects_zero_length_payload_serialization() {
    let record = BlobRecord {
        digest: [0u8; 32],
        bytes: vec![],
    };
    let result = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, 1024);
    assert!(result.is_ok(), "empty payload should be accepted");
}

// AFTER (option A — rename to reflect actual contract):
#[test]
fn encode_accepts_zero_length_payload() {
    let record = BlobRecord {
        digest: [0u8; 32],
        bytes: vec![],
    };
    let result = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, 1024);
    assert!(matches!(result, Ok(_)), "empty payload must round-trip: {result:?}");
}

// AFTER (option B — if rejection is the actual contract):
#[test]
fn encode_rejects_zero_length_payload_serialization() {
    let record = BlobRecord {
        digest: [0u8; 32],
        bytes: vec![],
    };
    let result = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, 1024);
    assert!(
        matches!(result, Err(JournalError::PayloadEmpty)),
        "zero-length payload must be rejected with PayloadEmpty, got {result:?}"
    );
}
```

### H-01 — `assert!(result.is_ok())` smoke tests in `hydrate_tests.rs`

```rust
// BEFORE (hydrate_tests.rs:128-133):
#[test]
fn validate_snapshot_metadata_accepts_matching_run() {
    let run = RunId::new(1);
    let result = validate_snapshot_metadata(run, EventSeq::new(0), run);
    assert!(result.is_ok(), "matching run should succeed, got {:?}", result);
}

// AFTER:
#[test]
fn validate_snapshot_metadata_accepts_matching_run() {
    let run = RunId::new(1);
    let result = validate_snapshot_metadata(run, EventSeq::new(0), run);
    assert_eq!(result, Ok(()), "matching run must return Ok(())");
}
```

### H-02 — `assert!(result.is_ok())` in `codec/tests.rs:2760-2773`

```rust
// BEFORE (codec/tests.rs:2751-2773):
#[test]
fn validate_replayed_event_with_zero_run_and_seq() {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(0),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let result = validate_replayed_event(RunId::new(0), EventSeq::new(0), &event);
    assert!(result.is_ok(), "zero run and seq should pass validation");
}

#[test]
fn validate_replayed_event_with_max_run_and_seq() {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(u64::MAX),
        seq: EventSeq::new(u64::MAX),
        attempt: 1,
        reason: None,
    };
    let result = validate_replayed_event(RunId::new(u64::MAX), EventSeq::new(u64::MAX), &event);
    assert!(result.is_ok(), "max run and seq should pass validation");
}

// AFTER:
#[test]
fn validate_replayed_event_with_zero_run_and_seq() {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(0),
        seq: EventSeq::new(0),
        attempt: 1,
        reason: None,
    };
    let validated = validate_replayed_event(RunId::new(0), EventSeq::new(0), &event)
        .expect("zero run/seq must pass validation");
    assert_eq!(validated.run, RunId::new(0));
    assert_eq!(validated.seq, EventSeq::new(0));
}

#[test]
fn validate_replayed_event_with_max_run_and_seq() {
    let event = JournalEvent::RunCancelled {
        run: RunId::new(u64::MAX),
        seq: EventSeq::new(u64::MAX),
        attempt: 1,
        reason: None,
    };
    let validated = validate_replayed_event(RunId::new(u64::MAX), EventSeq::new(u64::MAX), &event)
        .expect("max run/seq must pass validation");
    assert_eq!(validated.run, RunId::new(u64::MAX));
    assert_eq!(validated.seq, EventSeq::new(u64::MAX));
}
```

---

## Pattern Census

### `assert!(...is_ok()) / assert!(...is_err())` (BANNED in behavior assertions)

| Crate | Total matches | Top files |
|-------|---------------|-----------|
| `vb_storage/src` | 30 | `recovery/recovery_unit_tests.rs:789` (1), `codec/tests.rs:565,2760,2772` (3), `security_tests.rs:767,1056,1072` (3), `hydrate_tests.rs:132,155,178,208,214,220` (6), `edge_case_tests.rs:553` (1), `process_lock_tests.rs:219,227` (2), `tests/chunk_004.rs:166,270` (2), `tests/chunk_008.rs:173,181,185` (3), `tests/chunk_040.rs:212,227,268` (3), `recovery/replay/summary/tests.rs:467` (1), `type_tests.rs:196` (1), `tests/vb_core_atomic_admission_red.rs` (10 prop_assert), `tests/vb_god2f_classification_properties.rs` (2), `tests/vb_god2f_recovery_properties.rs` (2) |
| `vb_storage/tests` | 15 | `proptest_storage.rs` (mostly fixture unwrap, not banned), `recovery_property_tests.rs`, `proptest_journal_error_codes.rs` |
| `workspace_tests/tests` | 113 | `vb_test_core_workflow_slot_behavior.rs` (13), `vb_test_compile_parse_validate_behavior.rs` (9), `vb_test_validate_diagnostic_behavior.rs` (5), `vb_qi37_2_4_integration_budget_errors.rs` (5), `integration_validate_yaml_parsing.rs` (20+), `integration_compile_error_message_quality.rs` (3 — 2 are tautologies!), `integration_storage_runtime_validate_pipeline.rs` (5), `integration_boundary_inventory_evidence_validation.rs` (2), `integration_runtime_storage_fault_tolerance.rs` (1 — TAUTOLOGY), `contracts_production_binding.rs` (15), `doctor_storage_scan_decode_tests.rs` (13), `doctor_key_decode_tests.rs` (1), `postcard_envelope_wire_tests.rs` (6), `ipc_flag_matrix_tests.rs` (10+), `integration_validate_policy_enforcement.rs` (3) |
| `workspace_tests/benches` | 12 | `action_queuing.rs`, `array_queue.rs`, `cold_start.rs`, `pagination_cost.rs` |
| **TOTAL** | **~170** | (concentrated in `workspace_tests/tests/` — about 65% of all banned patterns) |

### Tautology assertions (`is_ok() || is_err()` or vice-versa)

| File:Line | Severity |
|-----------|----------|
| `workspace_tests/tests/integration_compile_error_message_quality.rs:376` | CRITICAL |
| `workspace_tests/tests/integration_compile_error_message_quality.rs:401` | CRITICAL |
| `workspace_tests/tests/integration_compile_error_message_quality.rs:424` | CRITICAL |
| `workspace_tests/tests/integration_runtime_storage_fault_tolerance.rs:215` | CRITICAL |
| `workspace_tests/benches/action_dispatch.rs:378` (in benchmark, lower severity) | OBSERVATION |
| **TOTAL** | **4 unique + 1 benchmark** |

### `let _ = ...` (silent error suppression)

| Crate | Total | Top files |
|-------|-------|-----------|
| `vb_storage/src` | 8 | `recovery/types/replay.rs:116,135,178` (3 — production match arms, not test), `recovery/replay/summary/slots/recovery.rs:71` (production) |
| `workspace_tests/tests` | ~80 | `timer_deadline_primitive_tests.rs` (~50 — `wheel.insert()` discarding results), `bdd_validation_tests.rs:1364-1376` (13 — variant existence), `ipc_flag_matrix_tests.rs:411,590,631,751,789` (~6), `cancel_kill_lattice_tests.rs:863,867` (2), `proptest_validation.rs:657` (1 — `let _ = validate(&parts)`, see L-04 of slice 3) |
| **TOTAL** | **~90** | Most are fixture construction or variant-existence checks (acceptable). ~10 are silent error suppression in test bodies. |

### `#[ignore]` / `#[should_panic]` / `sleep(` / `todo!()` / `unimplemented!()`

| Crate | Total | Notes |
|-------|-------|-------|
| `workspace_tests/tests` | 5 `#[ignore]` | `vb_qi37_25_quality_gates.rs:250`, `vb_c1s0_orchestration_runtime_tests.rs:593`, `vb_test_runtime_lifecycle_state_behavior.rs:469`, `vb_vt2f_direct_runtime_api_acceptance.rs:605`, `vb_njju_mutation_fuzz_property_closure.rs:224` — all commented with "Pre-existing issue" or "pending GAP" — HIGH severity. |
| `workspace_tests/tests` | 1 `sleep()` | `vb_a0t1_source_length_gate/support.rs:269` — bounded 50ms sleep in 60s timeout polling loop — LOW (acceptable). |
| `workspace_tests/tests` | 0 `#[should_panic]` | Clean. |
| **TOTAL** | 6 | (5 ignored behavior tests = HIGH; 1 sleep = LOW). |

### `lazy_static` / `OnceLock` / `static mut` / `thread_local!`

| Crate | Total | Notes |
|-------|-------|-------|
| All test files | **0** | No hidden shared mutable state in test code. The `OnceLock`/`Mutex` uses found in `vb_storage/src/journal/core.rs:125`, `vb_storage/src/queue/writer.rs:53`, `vb_storage/src/queue/loom_vb_mrwe_7.rs:8` are PRODUCTION code, not tests. CLEAN. |

### `panic!()` in test bodies (banned by rubric, but context-dependent)

| Crate | Total matches | Context |
|-------|---------------|---------|
| `workspace_tests/tests` | 68 | Almost all are `match other => panic!("expected X, got {other:?}")` arms. These are POSITIVE assertions that the wrong variant was matched, NOT banned-style panics. Acceptable. |

### Fuzz harnesses mis-categorized as `#[test]`

| File | Status |
|------|--------|
| `crates/vb_storage/src/kani_*.rs` (~80 files) | All properly gated by `#[cfg(all(kani, feature = "..."))]`. They are PROOF harnesses, not behavior tests. They do NOT appear as `#[test]` and are NOT executed by `cargo test` without the `kani` feature. CLEAN. |
| `crates/fuzz/fuzz_targets/*.rs` | Outside slice 2 scope. Verified they are correctly in `fuzz/` (not `crates/`). |
| **TOTAL fuzz/kani mis-categorized** | **0** |

---

## Mutation Gaps (top 5 most dangerous bugs the slice would NOT catch)

1. **`compile_workflow` returns `Ok(CompiledWorkflow::default())` for all inputs (compile error path deleted).**
   The 4 tautology assertions (`assert!(result.is_ok() || result.is_err())`) in
   `integration_compile_error_message_quality.rs:376,401,424` and the `assert!(result.is_ok() || result.is_err())`
   in `integration_runtime_storage_fault_tolerance.rs:215` would all pass. The `CompileError::DepthLimit`,
   `SequenceLimit`, `ScalarLimit` rejection paths in `vb_compile` could be deleted and these 4 tests
   would still pass. **File:Line:** production `crates/vb_compile/src/yaml_compiler.rs::compile`
   and `crates/vb_compile/src/error.rs::CompileError::*Limit` variants.

2. **`validate_snapshot_metadata` accepts any input (delete the matching-run check).**
   The 6 `assert!(result.is_ok())` in `hydrate_tests.rs:132,155,178,208,214,220` would all pass.
   The error-path tests (lines 136-148, 159-170, etc.) would still fail, so PARTIAL mutation
   (delete the success-path check, keep the failure-path check) is not caught. But if the failure
   path is also simplified, no test catches it. **File:Line:** production
   `crates/vb_storage/src/recovery/hydrate.rs::validate_snapshot_metadata` and friends.

3. **`process_lock::acquire` becomes a no-op.**
   `process_lock_prevents_dual_writers_same_directory` (line 141-162) and
   `process_lock_is_released_on_drop` (line 164-181) accept ALL outcomes (`Ok(_) => {}`,
   `Err(_) => {}`). Both tests would pass if `acquire` did nothing or always succeeded.
   The `.process.lock` file check on line 184-194 catches some mutations but not all
   (e.g., if `acquire` is a no-op, `lock_path.exists()` would still be true because some
   other code creates the file). **File:Line:** production
   `crates/vb_storage/src/process_lock.rs::acquire`.

4. **`encode_record` accepts any payload (including invalid magic/kind combinations) when the test only checks `is_ok()`.**
   `edge_case_tests.rs:547-554` (test name says "rejects" but asserts "accepts"),
   `codec/tests.rs:565,2760,2772` and `security_tests.rs:767` — all use `assert!(result.is_ok())`
   without verifying the encoded output. A mutation that returns `Ok(vec![0u8; N])` (empty
   or wrong-format bytes) for any input would pass these tests. The paired error-path tests
   catch the rejection, but the success-path tests don't verify the actual encoded structure.
   **File:Line:** production `crates/vb_storage/src/codec/encoder.rs::encode_record`.

5. **`discover_contracts` returns `Err(anyhow!("does not exist"))` for ALL errors.**
   `contracts_integration.rs:518-544` has 2 tests that assert `result.is_err()` then
   `err.contains("does not exist")` / `err.contains("not a directory")`. The success-path
   tests elsewhere check concrete values, so partial mutation is caught. But the two
   tests on lines 518-544 accept any error string that contains the expected phrase.
   If `discover_contracts` returned the wrong error with the right phrase (e.g., for a
   permission denied case, returned "does not exist"), the test would pass.
   **File:Line:** production `crates/xtask/src/contracts.rs::discover_contracts`.

---

## Top 5 Fixes (impact-per-effort)

### Fix 1 — Delete the 4 tautology `assert!(result.is_ok() || result.is_err())` lines
**Impact:** 4 test functions become real contract tests instead of `cargo test` noise.
**Effort:** 15 minutes. Each site needs to pick a real contract and write a `matches!` assertion.

```rust
// crates/workspace_tests/tests/integration_compile_error_message_quality.rs:374-376
// BEFORE:
    // Note: max_depth may not be enforced in all compile paths
    // This test verifies the limit type exists and can be configured
    assert!(result.is_ok() || result.is_err());
// AFTER:
    assert!(
        matches!(result, Err(vb_compile::CompileError::DepthLimit { depth, max }) if depth > max),
        "max_depth=1 must reject depth > 1, got {result:?}"
    );
```

### Fix 2 — Convert `process_lock_tests.rs:141-181` from "accepts all outcomes" to "asserts specific outcome"
**Impact:** 2 SECURITY-relevant behavior tests become real. `process_lock` is a SECURITY
contract — dual-writer prevention is a CBMC/Flux-relevant invariant.
**Effort:** 30 minutes.

```rust
// See full BEFORE/AFTER in the CRITICAL findings section above.
```

### Fix 3 — Rename `edge_case_tests.rs:547` to reflect actual contract
**Impact:** 1 test stops misleading future readers about what it covers.
**Effort:** 1 minute (rename). OR: if rejection is the actual contract, change the assertion.

### Fix 4 — Replace 6 `assert!(result.is_ok())` in `hydrate_tests.rs` with concrete value checks
**Impact:** 6 success-path tests gain concrete value assertions.
**Effort:** 20 minutes.

### Fix 5 — Decide on the 5 `#[ignore]` tests in workspace_tests
**Impact:** 5 behavior tests either get fixed (preferred) or removed.
**Effort:** Variable; depends on whether the contract is fixable.

```rust
// BEFORE (workspace_tests/tests/vb_test_runtime_lifecycle_state_behavior.rs:468):
    // Pre-existing issue: test fails with InvalidActionCompletion
    #[test]
    #[ignore]
    fn lifecycle_state_transitions_through_running_to_completed() { ... }

// AFTER (option A — fix and enable):
    #[test]  // no #[ignore]
    fn lifecycle_state_transitions_through_running_to_completed() { ... }

// AFTER (option B — remove if contract is unverifiable):
    // File deleted; contract covered by other tests.
```

---

## Disposition

| ID | Disposition | Rationale |
|----|-------------|-----------|
| C-01, C-02, C-03, C-04 | `blocker` | Pervasive logical tautologies and "match-all-outcomes" tests that pass for ANY production code. **REJECTED.** |
| H-01..H-10 | `blocker` | Each represents a tractable refactor; together they represent ~25 test sites that would let regressions pass silently. |
| M-01..M-11 | `owner_approved_debt` | Improvement opportunities; many are decorative-only (e.g., M-04 redundant `is_err` followed by `matches!`) and don't actually hide bugs. |
| L-01..L-08 | `owner_approved_no_action` | `panic!` in `other => panic!` arms is a positive assertion idiom (L-01); Kani harnesses are out of scope (L-05); controlled `sleep` in polling loop is acceptable (L-06). |
| O-01..O-05 | `owner_approved_no_action` | Observations on test design quality (mostly positive — `journal_event_tests.rs` and `bdd_validation_tests.rs` are exemplary). |

---

## Verdict

```
STATUS: REJECTED
```

**3 CRITICAL findings + 10 HIGH findings** that, if unaddressed, would let the most likely regressions
in the storage/workspace test surface go undetected:

1. **Tautology assertions** (4 sites): `assert!(result.is_ok() || result.is_err())` proves nothing.
   Fix: replace each with a concrete `matches!` or `assert_eq!`.
2. **"Accept-all-outcomes" tests** (`process_lock_tests.rs:141-181`): SECURITY-relevant tests that
   pass for ANY production behavior. Fix: assert specific `JournalError::ProcessLockHeld {..}`.
3. **Test name contradicts assertion** (`edge_case_tests.rs:547`): the test claims to verify
   rejection but asserts acceptance. Fix: rename OR change assertion.
4. **`assert!(result.is_ok())` smoke tests** (H-01 to H-07, ~25 sites): pass if production returns
   `Ok(_)` for any input. Fix: convert to `assert_eq!(result, Ok(SpecificValue))` or assert on
   the returned fields.
5. **`#[ignore]` on behavior tests** (5 sites): tests that document a contract the codebase cannot
   currently satisfy. Fix: fix the contract or remove the test.

The slice is otherwise strong: the BDD scenario file (`bdd_validation_tests.rs`), the journal event
decoder tests (`journal_event_tests.rs`), the contracts integration suite (`contracts_integration.rs`),
and the recovery boundary tests (`hydrate_tests.rs` failure-path tests) are exemplary.

Recommend:
(1) Fix the 4 tautologies in `integration_compile_error_message_quality.rs` and
`integration_runtime_storage_fault_tolerance.rs` (Fix 1, ~15 min).
(2) Convert `process_lock_tests.rs:141-181` to specific-outcome assertions (Fix 2, ~30 min).
(3) Rename or fix `edge_case_tests.rs:547` (Fix 3, ~1 min).
(4) Replace `assert!(result.is_ok())` in `hydrate_tests.rs` and `codec/tests.rs` (Fix 4, ~20 min).
(5) Triage the 5 `#[ignore]` tests (Fix 5, variable).

Total cleanup time: ~1-2 hours plus TDD sign-off on ignored tests.