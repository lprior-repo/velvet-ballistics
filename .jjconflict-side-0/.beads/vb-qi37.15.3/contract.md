bead_id: vb-qi37.15.3
bead_title: cli: Add trace command
phase: State 3
updated_at: 2026-05-12T00:00:00Z

# Contract Specification

## Context
- Feature: CLI `trace` command to show step-by-step execution trace for a submitted run
- Domain terms:
  - `run_id`: stable string identifier for a submitted workflow run
  - `db`: path to the Fjall journal storage directory
  - `TraceEntry`: structured record mapping a journal event to human/machine-readable fields
  - `JournalEvent`: persisted runtime event from the run's journal
  - `OutputFormat`: text (default), json, or jsonl
- Assumptions:
  - The run has been previously submitted via `submit` command
  - The journal storage is accessible and not corrupted
  - The trace command is read-only; it does not modify the journal
- Open questions: None

## Preconditions
- PRE-001: `run_id` argument is a valid run identifier string (non-empty, valid characters per `parse_run_id`).
- PRE-002: `--db` names an accessible Fjall journal directory containing the run's events.
- PRE-003: The run identified by `run_id` exists in the journal (has at least one event).

## Postconditions
- POST-001: `trace` outputs all journal events for the given `run_id` as ordered `TraceEntry` records.
- POST-002: Each `TraceEntry` contains `index`, `event_type`, `seq`, and variant-specific `extra_json` fields.
- POST-003: `--json` emits a single JSON object: `{"run_id": "...", "trace": [...entries], "total": N}`.
- POST-004: `--jsonl` emits one JSON object per trace entry followed by a final `{"total": N}` line.
- POST-005: Text format emits human-readable lines: `  [index] EventType step? (seq N)`.
- POST-006: If no events exist for the run, output is an empty trace array or `no events found for run <id>`.
- POST-007: Exit code is 0 on success (including empty trace), non-zero on storage or parse errors.

## Invariants
- INV-001: `build_trace` is pure: same `&[JournalEvent]` slice always produces identical `Vec<TraceEntry>` in same order.
- INV-002: Trace output is read-only; no journal writes occur during trace command execution.
- INV-003: Structured output fields (json/jsonl) are deterministic given the same journal events.

## Error Taxonomy
- ERR-001: Invalid `run_id` format -> `CliExitCode::InvalidArgument` with diagnostic.
- ERR-002: Journal directory not found or not readable -> `CliExitCode::StorageError` with diagnostic.
- ERR-003: Run ID not found in journal (no events) -> treated as empty trace (success, POST-006 applies).
- ERR-004: Journal read failure -> `CliExitCode::StorageError` with diagnostic.

## Contract Signatures
- `fn build_trace(events: &[JournalEvent]) -> Vec<TraceEntry>`
- `fn trace_one(idx: usize, event: &JournalEvent) -> TraceEntry`
- `fn cmd_trace(run_id: &str, db: &Path, output: OutputFormat) -> ExitCode`
- `fn read_journal_events(run_id: &str, db: &Path, output: OutputFormat) -> Result<Vec<JournalEvent>, ExitCode>`

## Verus-Owned Clauses
- INV-001: `build_trace` and `trace_one` are pure functions over `&[JournalEvent]` -> `Vec<TraceEntry>` with no side effects. Covered by unit tests in `commands_journal.rs` and property-based tests.
- PRE-001: `parse_run_id` validates run_id format. Covered by parser tests in `args.rs`.

## TLA+-Owned Clauses
- None required: trace is a read-only journal replay with no temporal state machine, no concurrency, no retry/lease logic, and no liveness conditions beyond "events eventually appear if they exist".

## Non-goals
- Trace does not replay or re-execute a run.
- Trace does not modify journal state.
- Trace does not support filtering or pagination (bounded output only).