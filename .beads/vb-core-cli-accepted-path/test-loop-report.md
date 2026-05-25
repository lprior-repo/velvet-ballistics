# Test Loop Report for DEFECT-12-02

**Bead:** vb-core-cli-accepted-path
**Date:** 2026-05-16
**State:** 7 (Test Planning)
**DEFECT:** DEFECT-12-02 - Test loop (States 7→8→9) never executed

## Isolation Verification

- `pwd -P`: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path`
- Path guard: **PASS** - Workdir is not source checkout and not nested under it

## Test Execution Summary

### PO-005: cli_integration (INT-CLI-001)

**Command:**
```bash
rustup run nightly-2026-04-28 cargo test --package velvet_ballistics --test cli_integration
```

**Result:** 82 PASSED, 4 FAILED

| Test | Result | Root Cause |
|------|--------|------------|
| `cli_run_strict_durability_writes_journal_events` | RED | Uses `run --durability strict` directly (bypass pattern) |
| `cli_ai_context_for_journaled_run_emits_compiled_ir_summary` | RED | Uses `run --durability journaled` directly (bypass pattern) |
| `cli_run_journaled_then_events_and_inspect_read_temp_db` | RED | Uses `run --durability journaled` directly (bypass pattern) |
| `cli_inspect_compiled_run_shows_status_and_event_count` | RED | Uses `run` for setup without proper artifact acceptance |

**All 4 failures show:** `runtime tick error: admission rejected: artifact invalid`

**Analysis:** These failures are **CORRECT BEHAVIOR**. The State 10 fix changed `Shard::new_with_journal` to use `StorageArtifactStore` for storage-backed journals, correctly rejecting artifacts that haven't been properly accepted first.

### PO-006: admission_evidence_integration (INT-CLI-002)

**Command:**
```bash
rustup run nightly-2026-04-28 cargo test --package velvet_ballistics --test admission_evidence_integration
```

**Result:** 8 PASSED, 0 FAILED

### PO-008: ir_artifact_admission (INT-BYPASS-001)

**Command:**
```bash
rustup run nightly-2026-04-28 cargo test --package velvet_ballistics --test ir_artifact_admission
```

**Result:** 8 PASSED, 0 FAILED

**Note:** `strict_direct_run` test target (mentioned in PO-008 command) does not exist. Existing `ir_artifact_admission` tests provide coverage for raw WorkflowParts, postcard IR, and unverified CompiledWorkflow rejection in strict mode.

### PO-009: proptest (PROP-DIGEST-001)

**Commands:**
```bash
rustup run nightly-2026-04-28 cargo test --test cli_envelope_proptest --all-features
rustup run nightly-2026-04-28 cargo test -p vb_storage --lib --all-features proptests
```

**Result:** 0 PASSED, 6 IGNORED (binary-only modules), 0 RUN (vb_storage proptests filtered)

## LETHAL Analysis

**NO LETHAL FINDINGS.**

The 4 RED cli_integration tests are NOT LETHAL:
- No hollow assertions (e.g., `assert!(x == x)`)
- No x==x proptests
- No `Ok(_) => {}` arms that swallow errors
- The failures are **genuine test failures** revealing that tests use an outdated bypass pattern

## Quarantined Findings

None. The RED tests are correctly failing - they need to be updated to use the proper submit-then-run pattern for strict/journaled policies.

## Required Test Updates

4 cli_integration tests need to be updated to:
1. First submit the workflow as an accepted artifact
2. Then run the workflow using the accepted artifact
3. Or use `Relaxed` durability policy for tests that don't need strict guarantees

## Conclusion

- **DEFECT-12-02 Status:** RESOLVED - Test loop executed
- **Test Loop Status:** COMPLETE_WITH_RED_TESTS_REQUIRING_UPDATE
- **Routing:** Route to State 8 for test updates, then State 9 for test review
