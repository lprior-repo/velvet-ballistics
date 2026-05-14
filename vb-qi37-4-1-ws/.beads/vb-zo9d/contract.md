bead_id: vb-zo9d
bead_title: cli/storage: Report journal trim eligibility in doctor
phase: 3
updated_at: 2026-05-09T20:30:00Z

# Contract Specification

## Context
- Feature: Extend the `doctor` CLI command to report journal trim eligibility without performing destructive trim operations.
- Domain terms:
  - **Trim eligibility**: Whether journal events for a run can be safely deleted based on durable snapshot coverage.
  - **Safe point**: The highest event sequence number covered by a durable snapshot for a run.
  - **Retention policy**: Rules that prevent trimming terminal runs among the N most recent terminal runs per workflow.
  - **Trim blocker**: A condition that prevents trimming (no snapshot, retention policy, etc.).
- Assumptions:
  - The storage trim API (bead vb-5h50) is complete and functional.
  - `FjallJournal::latest_durable_snapshot_seq` is available and correct.
  - Doctor command already supports `--json` / `--jsonl` output.
- Open questions: None.

## Preconditions
- P1: The database path exists and is readable.
- P2: The journal can be opened (FjallJournal::open succeeds).
- P3: The doctor command is invoked without any explicit destructive subcommand.

## Postconditions
- PO1: Doctor output includes a `trim_eligibility` check result.
- PO2: The trim eligibility check reports per-run status (eligible, blocked, no_snapshot).
- PO3: The trim eligibility check reports aggregate counts (total runs, eligible runs, blocked runs).
- PO4: The trim eligibility check reports the safe point (latest durable snapshot seq) for each run.
- PO5: If a run is blocked by retention policy, the check names the blocker.
- PO6: Doctor does NOT delete any journal events.
- PO7: Exit code is SUCCESS if the journal is healthy, regardless of trim eligibility.
- PO8: Exit code is StorageError if the journal cannot be opened.

## Invariants
- I1: Doctor command never mutates storage by default.
- I2: Structured and text diagnostics describe the same trim state.
- I3: Unsafe trim blockers fail closed (reported as blocked, not silently ignored).
- I4: The diagnostic method is pure read-only (no writes to fjall partitions).

## Error Taxonomy
- `TrimDiagnosticError::JournalOpenFailed` — when FjallJournal::open fails.
- `TrimDiagnosticError::SnapshotScanFailed` — when scanning snapshots fails.
- `TrimDiagnosticError::RunHeaderScanFailed` — when enumerating run headers fails.

## Contract Signatures

### Storage Layer (vb_storage)
```rust
/// Per-run trim eligibility status.
pub enum TrimEligibility {
    Eligible {
        run: RunId,
        safe_point: EventSeq,
        events_trimmable: u64,
    },
    Blocked {
        run: RunId,
        blocker: TrimBlocker,
    },
}

/// Reason a run cannot be trimmed.
pub enum TrimBlocker {
    NoDurableSnapshot,
    RetentionPolicy { retain_last_n_terminal: u32 },
}

/// Aggregate trim diagnostic for all runs in the journal.
pub struct TrimDiagnostic {
    pub runs: Vec<TrimEligibility>,
    pub total_runs: u64,
    pub eligible_runs: u64,
    pub blocked_runs: u64,
    pub total_events_trimmable: u64,
}

impl FjallJournal {
    /// Non-destructive trim eligibility diagnostic.
    /// Scans all runs and reports eligibility WITHOUT deleting anything.
    pub fn trim_eligibility_diagnostic(
        &self,
        policy: TrimPolicy,
    ) -> Result<TrimDiagnostic, JournalError>;
}
```

### CLI Layer (velvet_ballastics)
```rust
fn cmd_doctor(db: &Path, output: OutputFormat) -> ExitCode {
    // Existing checks: open_journal, strict_persist, append+read_back
    // NEW: trim_eligibility check — read-only, adds to checks vec
}
```

## Non-goals
- Do NOT add a new CLI subcommand for trim execution (trim already exists or is out of scope).
- Do NOT modify the existing trim policy defaults.
- Do NOT change the doctor exit code semantics for healthy journals.
