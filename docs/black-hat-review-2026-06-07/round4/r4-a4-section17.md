# Round 4 Agent A4 — Section 17 Dead-Letter Error Codes

**Reviewer:** black-hat-reviewer · **Date:** 2026-06-07 · **STATUS: REJECTED — SHIP-BLOCKER**

## Executive Summary

11 Section 17 runtime error codes are defined in the master spec, are not implemented in production, and are actively laundered by passing tests that document the gap as "Future." The most damaging specific case is SECRET_UNAVAILABLE being misrouted to ARTIFACT_MALFORMED_CODE (0x4017) — a security classification failure.

## Per-Code Findings

### FINDING-1: INPUT_MAPPING_FAILED — wrong exit code (HIGH 75/100)
- `map_runtime_inputs` (crates/vb_cli/src/run.rs:184-206) returns local InputMappingError
- stderr text is "INPUT_MAPPING_FAILED: input-bin decode failed" (correct)
- But mapped to `CliExitCode::CompileFailed = 3` instead of a runtime code

### FINDING-2: STEP_SKIPPED_REFERENCE — zero production sources (CRITICAL 95/100)
- Zero production occurrences outside documentation tests
- A step that is silently skipped due to a broken reference emits no diagnostic
- The run continues with stale or default values

### FINDING-3: WAIT_TIMEOUT — timer fire path silently succeeds (CRITICAL 90/100)
- `advance_after_timer_fire` (crates/vb_runtime/src/shard/helpers/timer.rs:24-52) handles timer fire by marking the step succeeded — no distinction between "waited full duration and timeout fired" vs "waited for an event that arrived"
- `InvalidTimerFire` returns `None` from runtime_code() — no runtime code at all
- Operators cannot distinguish a wait timeout from a runtime exception from an OOM

### FINDING-4: ASK_TIMEOUT — same path (CRITICAL 90/100)
- Identical to WAIT_TIMEOUT. No PendingTimerKind::Ask specific path that emits a distinct code.

### FINDING-5: FOR_EACH_ITEM_FAILED — body failure invisibility (HIGH 80/100)
- No body-failure aggregation in the for_each primitive
- A for_each with 1000 items, where item 500 fails, fails the entire run with a generic error
- No way to know which item failed or that the failure was inside a for_each

### FINDING-6: TOGETHER_BRANCH_FAILED — branch body invisibility (HIGH 80/100)
- Together runs are supposed to provide parallel-branch fault isolation
- A branch failure kills the entire together block with no indication of which branch failed

### FINDING-7: COLLECT_PAGE_FAILED — page fetch errors use generic time/item limit (MEDIUM 65/100)
- Page-order violation is silent in runtime_code()
- The error is propagated as a CoreError and converted via RuntimeError::Core → runtime_code() returns STORAGE_ERROR_RUNTIME_CODE (misclassified)

### FINDING-8: REDUCE_ITEM_FAILED — body failure invisibility (HIGH 80/100)
- Same as for_each

### FINDING-9: RESULT_REFERENCE_MISSING — zero production sources (CRITICAL 90/100)
- Zero production occurrences outside documentation tests
- A workflow referencing $steps.build.output where step build did not produce an output is killed with no diagnostic

### FINDING-10: REPLAY_DIVERGED — production path misroutes to storage error (CRITICAL 95/100)
- `RecoveryError::ReplayDivergence { step, detail }` exists at crates/vb_storage/src/recovery/types.rs:122-129
- There is NO `From<RecoveryError> for RuntimeError` implementation
- `cmd_replay` at crates/vb_cli/src/replay.rs:101-114 handles Err(_) (which includes ReplayDivergence) by returning `CliExitCode::StorageError = 5`
- The contract says exit code 8 (CliExitCode::ReplayDivergence) should be returned
- BDD test cli_vb_m214_bdd_scenarios.rs:382-395 is vacuous — only asserts command fails, not exit code 8

### FINDING-11: SECRET_UNAVAILABLE — misrouted to ARTIFACT_MALFORMED (CRITICAL 100/100)
- `JournalError::SecretUnavailable` exists at crates/vb_storage/src/error/mod.rs:207
- `diagnostic_code()` returns `ARTIFACT_MALFORMED_CODE = 0x4017` per crates/vb_storage/src/error/codes.rs:118,174
- A secret management system failure (the secret store is down, the secret was rotated out, the key was revoked) is logged to operators as "artifact malformed"
- **This is an information security failure** that breaks SOC2/ISO 27001 categorization

## Self-Laundering Tests

- crates/workspace_tests/tests/section17_runtime_code_reverse_parity.rs:35-50 hardcodes the 11 codes in `SECTION_17_UNMAPPED` and **asserts they must NOT appear** in runtime_code() output
- crates/workspace_tests/tests/section17_runtime_code_coverage_report.rs:159-217 documents the gap as "Future: X not yet implemented"
- vb_storage/tests/proptest_journal_error_codes.rs:393-397 calls the SecretUnavailable → ARTIFACT_MALFORMED_CODE = 0x4017 mapping correct. **It is not correct.**

## Quality Gates

| Gate | Result |
|---|---|
| cargo test -p vb_runtime | PASS (but tests don't cover missing paths) |
| cargo test -p vb_cli | PASS (cli_integration.rs:1780 only checks text substring) |
| Section 17 dead-letter gap | FAIL — 11/30 codes missing |
| BDD coverage | FAIL — Zero |

## Top 3 Worst Findings

1. **FINDING-11**: SECRET_UNAVAILABLE misrouted to ARTIFACT_MALFORMED — 100/100 severity. Security-relevant failure mode actively miscategorized in production logs, breaking audit trail semantics.

2. **FINDING-10**: REPLAY_DIVERGED returns exit code 5 instead of 8 — 95/100. The CliExitCode::ReplayDivergence variant exists; verify uses it; replay doesn't. Monitoring/alerting broken.

3. **FINDING-2**: STEP_SKIPPED_REFERENCE zero production sources — 95/100. A step silently skipped due to broken reference emits no diagnostic. The run continues with stale state.

## Required Repair Actions

1. **CRITICAL FINDING-11**: Modify `JournalError::SecretUnavailable` to either have its own `DiagnosticCode` (e.g. 0x1507) or be passed through to a new `RuntimeError::SecretUnavailable` variant.
2. **CRITICAL FINDING-10**: Add a `From<RecoveryError> for CliExitCode` (or a dedicated match arm) so `RecoveryError::ReplayDivergence { .. }` returns `CliExitCode::ReplayDivergence = 8`. Strengthen the BDD test.
3. **CRITICAL FINDING-3,4**: Modify `advance_after_timer_fire` to distinguish `PendingTimerKind::Wait` from `PendingTimerKind::Ask`. Add `RuntimeError::WaitTimeout { step }` and `RuntimeError::AskTimeout { step }` variants.
4. **HIGH FINDING-1**: Move `InputMappingError` enum into `vb_runtime` with a `runtime_code()` returning "INPUT_MAPPING_FAILED".
5. **HIGH FINDING-2,9**: Either implement runtime reference validation that emits these codes, or remove them from master:723,734.
6. **HIGH FINDING-5,6,8**: Add error context to the engine failure path. A simple change in `apply_terminal_failed` to include the outer iteration step index.
7. **HIGH FINDING-7**: Add `RuntimeError::CollectPageFailed { step, kind }` variant with mapping to "COLLECT_PAGE_FAILED".
8. **CRITICAL**: Delete the self-laundering tests. Either remove the UNMAPPED / PARTIALLY_MAPPED sections and have the tests fail loudly when codes are missing.
9. **MEDIUM**: Audit `vb_storage/tests/proptest_journal_error_codes.rs:393-397`. Fix the mapping first, then fix the test.

## Verdict: SHIP-BLOCKER

Until at least items 1, 2, 3, 4, 8, 9 are completed, the system does not satisfy the master contract for Section 17 and cannot be shipped.
