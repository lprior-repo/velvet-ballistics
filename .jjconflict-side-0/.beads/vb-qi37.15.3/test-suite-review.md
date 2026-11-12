bead_id: vb-qi37.15.3
bead_title: cli: Add trace command
phase: 9
updated_at: 2026-05-18T00:00:00Z
attempt: 1

# test-suite-review.md

## VERDICT: APPROVED (with advisory notes)

### Tier 0 — Static

**[PASS]** Banned pattern scan
- `assert!(result.is_ok())` / `assert!(result.is_err())` found in `main_tests.rs:882,890,898,906,915,923,929,934` — see advisory below
- No `let _ =` or `.ok();` silent suppression in test assertions
- No `#[ignore]` tests found
- No `sleep` in tests

**[PASS]** Determinism/evidence scan
- No `static mut`, `lazy_static!`, `once_cell.*Mutex` in test code
- No shared mutable state between tests

**[PASS]** Mock interrogation
- No `mockall` usage in test files

**[PASS]** Integration test purity
- `cli_trace_integration.rs` uses only public API via `std::process::Command`
- No `use crate::` imports in `/tests/`

**[PASS]** Error variant completeness
- `CliExitCode::ValidationFailed` (exit 1) — tested by `parse_run_id_rejects_*` + `cmd_trace_invalid_run_id_format`
- `CliExitCode::StorageError` (exit 5) — tested by `cmd_trace_invalid_db_path`, `read_journal_events_returns_storage_error_when_dir_not_found`
- `CliExitCode::Success` (exit 0) — tested by all `assert_cli_success` tests

**[PASS]** Density audit
- Commands_journal.rs: 5 pub functions (`build_trace`, `trace_one`, `analyze_retry`, `analyze_resume`, helper types) — 25+ unit tests (incl. 18 variant tests + proptest)
- Ratio: 5× ✓

**[ADVISORY]** `assert!(result.is_ok())` / `assert!(result.is_err())` pattern in `main_tests.rs`
- Files: `main_tests.rs:882,890,898,906,915,923,929,934`
- These tests ARE followed by exact-value assertions in most cases (see Tier 1 evidence), so the evidence is not hollow
- However, the bare `assert!(result.is_ok())` violates strict assertion doctrine
- Advisory: prefer `let Ok(v) = result else { return; }; assert_eq!(v.get(), expected)` pattern
- Not LETHAL because downstream assertions do assert exact values — documented as Tier 0 advisory only

---

### Tier 1 — Execution

**[PASS]** Test compile: pass
```
cargo test --all-features --no-run → compiled successfully
```

**[FAIL — EXPECTED RED PHASE]** nextest: 562 passed, **2 failed**, 1 skipped
- `parse_run_id_rejects_zero` — **FAIL_FIRST** — implementation does not reject zero
- `read_journal_events_returns_storage_error_when_dir_not_found` — **FAIL_FIRST** — implementation behavior differs from test expectation (FjallJournal creates dir or returns empty gracefully vs. returning StorageError)

Both failures are **implementation gaps**, not test gaps. The tests correctly encode the expected behavior per the contract.

```
Summary: 564 tests run: 562 passed, 2 failed, 1 skipped
```

**[PASS]** Ordering probe: consistent
- Tests pass with `--test-threads=1` and `--test-threads=8` identically

**[N/A]** Insta: not present

---

### Tier 2 — Coverage

**NOT EXECUTED** — Coverage gate requires passing Tier 1. With 2 expected red-phase failures, coverage measurement is deferred until implementation fixes are applied. Evidence from test-writer shows:

- `trace_one` all 18 variant tests provide branch coverage of the match expression
- `build_trace` length preservation and determinism tests cover the pure function
- `parse_run_id` boundary tests cover the validation logic

Coverage will be re-measured after State 10 implementation fixes.

---

### Tier 3 — Mutation

**DEFERRED** — Mutation gate requires a passing test suite. Kill rate measurement deferred to post-implementation State 11 execution.

---

## FINDINGS

### LETHAL FINDINGS
None.

### MAJOR FINDINGS (0)
None.

### MINOR FINDINGS (2/5 threshold)
1. `main_tests.rs:882,890,898,906,915,923,929,934` — `assert!(result.is_ok())` / `assert!(result.is_err())` bare boolean assertion pattern. Downstream exact-value assertions exist in most cases, so evidence is not hollow. Advisory for test-writer: prefer direct pattern matching with `let Ok(v) = result else { return; }; assert_eq!(v.get(), expected)`.
2. `cli_trace_integration.rs:9` — unused import `vb_core::value::SlotValue`. Non-blocking (test compiles and runs).

---

## RED PHASE FAILURES (Expected — Not Blockers)

| Test | Expected Behavior | Actual Behavior | Owner State |
|---|---|---|---|
| `parse_run_id_rejects_zero` | `parse_run_id("0") → Err(ValidationFailed)` | `Ok(RunId(0))` — `RunId::new(0)` is valid | State 10 |
| `read_journal_events_returns_storage_error_when_dir_not_found` | exit code 5 (StorageError) | exit code 0 (empty trace) | State 10 |

Both are correct test specifications encoding contract POST-003/POST-005 requirements. The implementation must be corrected in State 10.

---

## MANDATE

**Test suite is APPROVED for red-phase execution.**

State 10 (holzman-rust) must fix two implementation gaps before the suite goes green:
1. Add `id != 0` validation to `parse_run_id` — test `parse_run_id_rejects_zero` will then pass
2. Clarify or fix `read_journal_events` / FjallJournal open behavior to return `StorageError` when the journal directory does not exist — test `read_journal_events_returns_storage_error_when_dir_not_found` will then pass

After implementation fixes, re-run Tier 1 through Tier 3 gates to confirm green suite.
