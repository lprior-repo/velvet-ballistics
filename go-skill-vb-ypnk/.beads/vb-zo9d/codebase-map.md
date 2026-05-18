bead_id: vb-zo9d
bead_title: cli/storage: Report journal trim eligibility in doctor
phase: 2
updated_at: 2026-05-09T20:25:00Z

## Codebase Map

### Relevant Files

#### Doctor CLI (`crates/velvet_ballastics/src/main.rs`)
- `cmd_doctor(db: &Path, output: OutputFormat) -> ExitCode` at line 3644
- Currently performs 3 checks: open_journal, strict_persist, append+read_back_event
- Outputs either text (`outln!`) or structured JSON (`json_out` / `json_error`)
- No trim eligibility reporting yet
- Uses `CliExitCode::StorageError` on failure

#### Storage Trimming (`crates/vb_storage/src/trimming.rs`)
- `TrimPolicy` { skip_noop_runs, retain_last_n_terminal: u32 } — default retain_last_n_terminal=10
- `TrimError` variants: `NoDurableSnapshot { run }`, `RetentionPolicyBlocks { run }`, `IncompleteTrim { deleted_count }`
- `FjallJournal::latest_durable_snapshot_seq(run) -> TrimResult<Option<EventSeq>>` — non-destructive, public
- `FjallJournal::trim_events_for_run(run, policy) -> TrimResult<TrimmedRunResult>` — DESTRUCTIVE, public
- `FjallJournal::trim_all_eligible_runs(policy)` — DESTRUCTIVE, public
- `FjallJournal::has_terminal_event(run)` — private, used for retention check
- `FjallJournal::check_retention_policy(run, policy)` — private, returns Err if blocked
- `TrimmedRunResult` { run, deleted_count, cutoff_seq, status }
- `TrimStatus` { Trimmed, NoOp }

#### UI Model (`crates/vb_ui_model/src/lib.rs`)
- `JournalDoctorPanel` { run_event_count, snapshot_seq, tail_seq, corrupt_records, trim_recommendation, digest_checks }
- `TrimRecommendation` enum: `NotNeeded`, `Recommended { tail_seq, snapshot_seq }`, `Critical { tail_seq, snapshot_seq }`
- Used by `vb_ui_snapshot/src/fixtures.rs` for UI fixture data

#### Storage Exports (`crates/vb_storage/src/lib.rs`)
- Re-exports: `TrimError`, `TrimPolicy`, `TrimResult`, `TrimStatus`, `TrimmedRunResult`
- `FjallJournal` is the main journal handle

### Gap Analysis

The doctor command needs a **non-destructive** trim eligibility diagnostic. The existing
storage API only provides:
1. `latest_durable_snapshot_seq()` — tells us the safe point, but only for one run
2. `trim_events_for_run()` — actually performs deletion

What is missing:
- A diagnostic-only method that iterates runs and reports trim eligibility WITHOUT deleting anything
- Per-run eligibility status: eligible / blocked_by_no_snapshot / blocked_by_retention
- Aggregate stats: total runs, eligible runs, blocked runs, total events trimmable
- Doctor output integration: new check in `cmd_doctor` that adds a "trim_eligibility" check object

### Key Design Decisions
1. The diagnostic must be pure read-only (no mutations, safe for incident triage)
2. Should reuse existing `latest_durable_snapshot_seq` and `has_terminal_event` logic
3. Should report per-run blockers and aggregate counts
4. Must work with both text and JSON output paths
5. Exit code should distinguish healthy-but-trimmable vs healthy-and-clean vs error states
