# Proof Evidence — vb-qi37.17.1: cli: Add incident command

**Bead**: vb-qi37.17.1
**Agent**: proof-writer
**Date**: 2026-05-18
**Source**: `/home/lewis/src/go-skill-vb-qi37.17.1`

## Evidence summary

**No formal proof artifacts were produced.** The proof strategy determined that formal verification (TLA+, Verus, Kani, Miri, Flux, Loom, proptest, fuzz) is `not_applicable` to this bead. The correctness proof is the **test suite** — 16 test obligations exercising every contract clause.

## Proof obligations (from proof-obligations.planned.jsonl)

### Static-scan obligations (compile + structural correctness)

| Obligation | Contract clause | Claim | Layer | Status |
|------------|----------------|-------|-------|--------|
| COMPILE-001 | INV-005 | All `recover_full_journal` call sites pass 5 arguments | static-scan | PLANNED — awaiting implementation |
| COMPILE-002 | INV-005 | All `replay_events` call sites pass 3 arguments | static-scan | PLANNED — awaiting implementation |
| UNWRAP-001 | INV-001 | `serde_json` serialization uses `match Result`, not `unwrap_or_default` | static-scan | PLANNED — awaiting implementation |
| UNWRAP-002 | INV-001 | `as_str().unwrap_or()` on `Option<&str>` is zero-panic | waiver | WAIVED — contract.md confirms safety |
| DEAD-001 | INV-006 | Dead `parse_incident` in `args/run_db.rs` removed | static-scan | PLANNED — awaiting implementation |

### Unit-test obligations: `build_incident_report`

| Obligation | Contract clause | What it proves | Test name | Status |
|------------|----------------|----------------|-----------|--------|
| T-001 | POST-001 | Empty events → `failure_found: false`, `failure_code: ""`, empty side_effects | `test_build_incident_report_empty_events` | PLANNED — awaiting test-writer |
| T-002 | POST-001 | `StepStarted` + `RunFailedEvent` → `failure_found: true`, `failure_code: "RunFailed"`, `failed_at_step` set | `test_build_incident_report_run_failed` | PLANNED — awaiting test-writer |
| T-003 | POST-001 | `ActionCompletedEvent` + `RunFailedEvent` → side_effects has one confirmed entry | `test_build_incident_report_action_completed` | PLANNED — awaiting test-writer |
| T-004 | POST-001 | `ActionFailedEvent` + `RunFailedEvent` → side_effects has one failed entry | `test_build_incident_report_action_failed` | PLANNED — awaiting test-writer |
| T-005 | POST-001 | `ActionCompleted` + `ActionFailed` + `RunFailed` → two side_effects entries | `test_build_incident_report_multiple_actions` | PLANNED — awaiting test-writer |
| T-006 | POST-001 | `RunCancelled` → `failure_found: true`, `failure_code: "RunCancelled"` | `test_build_incident_report_run_cancelled` | PLANNED — awaiting test-writer |
| T-007 | POST-001 | Multiple `StepStarted` → `failed_at_step` is LAST step before failure | `test_build_incident_report_failed_at_step` | PLANNED — awaiting test-writer |
| T-008 | INV-001 | Unknown/ignored `JournalEvent` variants → no panic | `test_build_incident_report_unknown_variants` | PLANNED — awaiting test-writer |

### Unit-test obligations: `build_repair_hints`

| Obligation | Contract clause | What it proves | Test name | Status |
|------------|----------------|----------------|-----------|--------|
| T-009 | POST-002 | `RunFailed` + empty side_effects + no step → 1 hint | `test_build_repair_hints_run_failed_empty` | PLANNED — awaiting test-writer |
| T-010 | POST-002 | `RunFailed` + side_effects + step → 3 hints | `test_build_repair_hints_run_failed_full` | PLANNED — awaiting test-writer |
| T-011 | POST-002 | `RunCancelled` + empty side_effects → 1 hint | `test_build_repair_hints_run_cancelled_empty` | PLANNED — awaiting test-writer |
| T-012 | POST-002 | `RunCancelled` + side_effects → 2 hints | `test_build_repair_hints_run_cancelled_full` | PLANNED — awaiting test-writer |
| T-013 | POST-002 | Unknown failure code → 0 hints | `test_build_repair_hints_unknown_code` | PLANNED — awaiting test-writer |

### Integration-test obligations: `cmd_incident`

| Obligation | Contract clause | What it proves | Test name | Status |
|------------|----------------|----------------|-----------|--------|
| T-014 | POST-003, INV-003 | Failed run → JSON output with `failure_code: "RunFailed"`, valid JSON | `test_failed_run_json_output` | PLANNED — awaiting test-writer |
| T-015 | POST-003, INV-002 | Non-existent run → structured JSON error, no stack trace | `test_missing_run_error_output` | PLANNED — awaiting test-writer |
| T-016 | POST-004 | Run with no failure event → exit code indicates no incident | `test_no_failure_run_exit_code` | PLANNED — awaiting test-writer |

### Manual-QA obligation

| Obligation | Contract clause | What it proves | Method | Status |
|------------|----------------|----------------|--------|--------|
| QA-001 | INV-002 | No stack traces in any output path (JSON, JSONL, Text, error) | Manual: `velvet-ballistics incident <test_run_id> --db <test_db> --format json` | PLANNED — awaiting QA agent |

## Coverage matrix

| Contract clause | Covered by | Obligations |
|----------------|------------|-------------|
| PRE-001 | T-001 through T-016 (implicit via input validation) | T-001–T-016 |
| PRE-003 | T-001 through T-008 | T-001–T-008 |
| PRE-004 | T-009 through T-013 | T-009–T-013 |
| POST-001 | T-001 through T-008 | T-001–T-008 |
| POST-002 | T-009 through T-013 | T-009–T-013 |
| POST-003 | T-014, T-015 | T-014–T-015 |
| POST-004 | T-016 | T-016 |
| INV-001 | T-008, UNWRAP-001, UNWRAP-002 | T-008, UNWRAP-001, UNWRAP-002 |
| INV-002 | T-015, QA-001 | T-015, QA-001 |
| INV-003 | T-014 | T-014 |
| INV-004 | T-014 (JSON key ordering) | T-014 |
| INV-005 | COMPILE-001, COMPILE-002 | COMPILE-001, COMPILE-002 |
| INV-006 | DEAD-001 | DEAD-001 |

## Assumptions

1. `JournalEvent` variants used in tests match the production enum in `vb_storage`.
2. `StepIdx::get()` returns `u16`; `ActionId::get()` returns `String`.
3. `serde_json::json!` macro is available in `vb_cli`.
4. The 56 E0061 compile errors are all from `recover_full_journal` (5-arg) and `replay_events` (3-arg) call-site arity mismatches — fixable by appending `, &[]`.
5. `serde_json::to_string_pretty` / `to_string` failures are vanishingly rare on well-constructed `serde_json::Value` — but INV-001 requires a `Result`-based error path rather than `unwrap_or_default`.

## Conclusion

The proof-writer role for vb-qi37.17.1 is **COMPLETE**. No formal proof artifacts exist because:

- The code path is pure-functional + sequential CLI I/O.
- There is no unsafe code, no concurrency, no temporal behavior.
- The 16 test obligations (T-001 through T-016) provide stronger, cheaper, more actionable evidence than any formal verifier.
- The 5 static-scan obligations (COMPILE-001, COMPILE-002, UNWRAP-001, UNWRAP-002, DEAD-001) are binary properties verified by cargo tooling.

All 22 obligations are tracked in `proof-obligations.planned.jsonl`. The test-writer (state 8) must produce the 16 test artifacts that form the actual proof of correctness.
