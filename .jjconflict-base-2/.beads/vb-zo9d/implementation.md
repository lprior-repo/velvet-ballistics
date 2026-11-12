bead_id: vb-zo9d
bead_title: cli/storage: Report journal trim eligibility in doctor
phase: 6
updated_at: 2026-05-09T21:05:00Z

# Implementation Summary

## Changes Made

### 1. vb_storage/src/trimming.rs

**New types added:**
- `TrimEligibility` enum: `Eligible { run, safe_point, events_trimmable }` / `Blocked { run, blocker }`
- `TrimBlocker` enum: `NoDurableSnapshot` / `RetentionPolicy { retain_last_n_terminal }`
- `TrimDiagnostic` struct: `runs, total_runs, eligible_runs, blocked_runs, total_events_trimmable`

**New methods added:**
- `FjallJournal::trim_eligibility_diagnostic(policy) -> Result<TrimDiagnostic, JournalError>`
  - Non-destructive diagnostic that scans all runs and reports eligibility
  - Uses `latest_durable_snapshot_seq` to find safe points
  - Uses `check_retention_policy` (now `pub(crate)`) to detect retention blockers
  - Uses new `count_trimmable_events` helper to count deletable events per run
- `FjallJournal::count_trimmable_events(run, safe_point) -> Result<u64, JournalError>`
  - Pure read-only scan of event prefix, counts events with seq < safe_point

**Visibility changes:**
- `has_terminal_event`: `private -> pub(crate)`
- `check_retention_policy`: `private -> pub(crate)`

### 2. vb_storage/src/error.rs

- Added `JournalError::Trim(Box<TrimError>)` variant to support `From<TrimError>` conversion
- Implemented `From<TrimError> for JournalError` that:
  - Maps `TrimError::Fjall` -> `JournalError::Fjall`
  - Maps `TrimError::Journal` -> inner `JournalError` (unwrapping)
  - Maps all other variants -> `JournalError::Trim(Box::new(...))`

### 3. vb_storage/src/lib.rs

- Added re-exports: `TrimBlocker`, `TrimDiagnostic`, `TrimEligibility`

### 4. velvet_ballistics/src/main.rs

- Extended `cmd_doctor` with Check 4: `trim_eligibility`
  - Calls `journal.trim_eligibility_diagnostic(TrimPolicy::default())`
  - Formats per-run results into JSON check object with `total_runs`, `eligible_runs`, `blocked_runs`, `total_events_trimmable`, `runs` array
  - Text mode prints summary line and per-run details
  - On error, returns `CliExitCode::StorageError`

### 5. velvet_ballistics/tests/cli_integration.rs

- Added 4 integration tests:
  - `cli_doctor_json_includes_trim_eligibility_check`
  - `cli_doctor_text_reports_trim_eligibility`
  - `cli_doctor_returns_success_for_healthy_journal_with_trim_recommended`
  - `cli_doctor_returns_storage_error_for_unreadable_path`

### 6. vb_storage/src/trimming.rs (tests)

- Added 8 unit tests for diagnostic behavior:
  - `diagnostic_returns_eligible_and_blocked_runs`
  - `diagnostic_reports_correct_safe_point_and_trimmable_count`
  - `diagnostic_blocks_run_without_durable_snapshot`
  - `diagnostic_blocks_recent_terminal_run_under_retention`
  - `diagnostic_allows_non_terminal_run_despite_retention`
  - `diagnostic_does_not_delete_events`
  - `diagnostic_is_idempotent`
  - `diagnostic_returns_empty_for_empty_journal`

## Contract Mapping

| Contract Clause | Implementation | Test |
|---|---|---|
| PO1 (trim_eligibility check) | cmd_doctor adds check to JSON/text | cli_doctor_json_includes_trim_eligibility_check |
| PO2 (per-run status) | TrimEligibility enum in diagnostic | diagnostic_returns_eligible_and_blocked_runs |
| PO3 (aggregate counts) | TrimDiagnostic fields | cli_doctor_json_includes_trim_eligibility_check |
| PO4 (safe point) | latest_durable_snapshot_seq | diagnostic_reports_correct_safe_point_and_trimmable_count |
| PO5 (retention blocker) | check_retention_policy | diagnostic_blocks_recent_terminal_run_under_retention |
| PO6 (no mutation) | read-only scan | diagnostic_does_not_delete_events |
| PO7 (SUCCESS exit) | cmd_doctor returns SUCCESS | cli_doctor_returns_success_for_healthy_journal_with_trim_recommended |
| PO8 (StorageError) | early return on open failure | cli_doctor_returns_storage_error_for_unreadable_path |
| I1 (read-only) | no writes in diagnostic | diagnostic_does_not_delete_events |
| I2 (parity) | same data in JSON and text | cli_doctor_text_reports_trim_eligibility |
| I3 (fail closed) | blockers reported explicitly | diagnostic_blocks_run_without_durable_snapshot |
| I4 (pure diagnostic) | snapshot-based iteration | diagnostic_is_idempotent |

## Known Limitations

- The `vb_storage` crate has 132 pre-existing compilation errors in unrelated test modules
  (`src/tests.rs`, `src/vb_2bok_durability_gate_tests.rs`, `src/recovery/vb_h6ix_tests.rs`).
  These errors exist on the main branch and are unrelated to bead vb-zo9d.
  The library compiles successfully; only test compilation is affected.
