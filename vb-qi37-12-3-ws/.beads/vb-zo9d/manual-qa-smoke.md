bead_id: vb-zo9d
bead_title: cli/storage: Report journal trim eligibility in doctor
phase: 7
updated_at: 2026-05-09T21:30:00Z

# Manual QA Smoke Report

## Test Environment
- Binary: velvet_ballastics (debug build)
- Workspace: vb-zo9d-ws
- Test database paths: /tmp/vb-doctor-empty-test, /tmp/vb-doctor-qa-db2

## Interface Surface
- `doctor --db <path>` (text output)
- `doctor --db <path> --json` (JSON output)
- `doctor --db <path> --jsonl` (JSONL output)

## Test Matrix

### Happy Path 1: Doctor text on empty journal
```bash
$ ./target/debug/velvet-ballistics doctor --db /tmp/vb-doctor-empty-test
```
**Expected:** Trim eligibility summary + all checks passed
**Actual:**
```
doctor: trim eligibility — 0 total, 0 eligible, 0 blocked, 0 events trimmable
doctor: all checks passed
```
**Exit code:** 0
**Status:** PASS

### Happy Path 2: Doctor JSON on empty journal
```bash
$ ./target/debug/velvet-ballistics doctor --db /tmp/vb-doctor-empty-test --json
```
**Expected:** JSON with trim_eligibility check containing totals and runs array
**Actual:**
```json
{
  "checks": [
    {"check": "open_journal", "status": "pass", ...},
    {"check": "strict_persist", "status": "pass", ...},
    {"check": "append_event", "status": "pass", ...},
    {"check": "read_back_event", "status": "pass", ...},
    {
      "check": "trim_eligibility",
      "status": "pass",
      "message": "trim eligibility: 0 total, 0 eligible, 0 blocked, 0 events trimmable",
      "total_runs": 0,
      "eligible_runs": 0,
      "blocked_runs": 0,
      "total_events_trimmable": 0,
      "runs": []
    },
    {"check": "all", "status": "pass", ...}
  ],
  "success": true
}
```
**Exit code:** 0
**Status:** PASS

### Error Path: Doctor on unreadable path
```bash
$ ./target/debug/velvet-ballistics doctor --db /nonexistent/path/to/db
```
**Expected:** Error message + non-zero exit code
**Actual:**
```
FAIL: cannot open journal at /nonexistent/path/to/db: fjall journal operation failed: FjallError: Io(Os { code: 13, kind: PermissionDenied, message: "Permission denied" })
```
**Exit code:** 5
**Status:** PASS

### Integration Test Suite
```bash
$ cargo test -p velvet_ballastics --test cli_integration cli_doctor
```
**Results:**
- `cli_doctor_json_includes_trim_eligibility_check` — PASS
- `cli_doctor_text_reports_trim_eligibility` — PASS
- `cli_doctor_returns_success_for_healthy_journal_with_trim_recommended` — PASS
- `cli_doctor_returns_storage_error_for_unreadable_path` — PASS

**Status:** 4/4 PASS

## Findings

No critical or major findings. All tested paths behave as expected.

### Observations
1. Doctor creates the database directory if it doesn't exist (FjallJournal::open behavior).
2. The trim eligibility check is read-only and safe to run during incident triage.
3. Text output format includes a summary line followed by per-run details.

## Known Limitations
- vb_storage unit tests cannot be executed due to 66 pre-existing compilation errors
  in `vb_h6ix_tests.rs` and related files (mismatch between test expectations and
  the `events.rs` enum definition). These errors exist in the parent commit and are
  unrelated to bead vb-zo9d.

## Summary

| Category | Tests | Pass | Fail |
|---|---|---|---|
| Happy path | 2 | 2 | 0 |
| Error path | 1 | 1 | 0 |
| Integration | 4 | 4 | 0 |
| **Total** | **7** | **7** | **0** |

STATUS: PASS
