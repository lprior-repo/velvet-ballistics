# Contract Specification — vb-oaom: cli: Add runtime ai context packet command

## Context

- **Feature**: `ai-context` CLI subcommand — emits a bounded, redacted AI-safe context packet for a specific run.
- **Domain terms**:
  - `AiContextPacket` — structured JSON object describing a run's state, artifacts, and safe next actions.
  - `RunId` — parsed from CLI argument as `u64`.
  - `Journal` — `FjallJournal` backing store opened via `--db <path>`.
  - `RedactionStatus` — whether a slot value is `Clean`, `DerivedFromSecret`, or `Secret`; secret-tainted slots emit `[REDACTED]` in the packet.
  - `SuggestedCommand` — a real `velvet-ballastics` CLI command that is safe to recommend.
- **Assumptions**:
  - The `ai-context` command is a cold CLI path; no live runtime shard is required.
  - Journal events and run headers are loaded from the Fjall keyspace.
  - The packet is read-only; no run state is modified.
  - The implementation uses `serde_json` only for CLI output formatting; runtime core remains postcard + Fjall.
- **Open questions**: None. The existing `commands_ai_context.rs` implementation is the authoritative reference.

## Preconditions

- PRE-001: `run_id` argument must parse as `u64` via `RunId::new`.
- PRE-002: `--db` path must exist and be openable as `FjallJournal`.
- PRE-003: The identified run must have at least one journal event in the journal (non-empty event trail).

## Postconditions

- POST-001: Output is a valid JSON object conforming to the `AiContextPacket` schema with fields: `schema_version`, `kind`, `run_id`, `workflow`, `journal_event_trail`, `action_contracts`, `trace_ring_snapshot`, `suggested_next_cli_commands`.
- POST-002: `workflow` field contains at least `digest`, `compiled_ir` (with availability flag), and `referenced_actions`.
- POST-003: Every slot value in `journal_event_trail` that is `Secret`- or `DerivedFromSecret`-tainted (per snapshot taint table) is replaced with `[REDACTED]`; all other slots render as their `SlotValue::to_string()` representation or `[UNDECODED]` on decode failure.
- POST-004: `suggested_next_cli_commands` contains only real `velvet-ballastics` CLI commands: `inspect`, `events`, and status-dependent extras (`incident`/`retry` for failed/cancelled, `trace`/`resume` for running, `replay` for finished).
- POST-005: `action_contracts` lists unique action IDs inferred from `Do` nodes in compiled IR and from journal events, each annotated with `contract_status: "inferred_from_compiled_ir_and_journal"`.
- POST-006: On a run-not-found error (zero events), CLI exits with `CliExitCode::ValidationFailed` and outputs a structured error JSON containing `"code": "RUN_NOT_FOUND"`.

## Invariants

- INV-001: AI context packet is **read-only** — the command never mutates run state, journal, or any artifact.
- INV-002: Packet size is **bounded** — `journal_event_trail` contains all events from the journal; `suggested_next_cli_commands` is a fixed-length list derived from run status (max 4 commands).
- INV-003: Every recommended command in `suggested_next_cli_commands` maps to an existing real CLI subcommand (`inspect`, `events`, `incident`, `retry`, `trace`, `resume`, `replay`).
- INV-004: `redaction_status` is **always explicit** — slots that cannot be decoded do not silently pass; they emit `[UNDECODED]` with the slot index cited.

## Error Taxonomy

- `Error::InvalidRunId` — `run_id` argument is not a valid `u64` decimal string.
- `Error::JournalOpen` — `--db` path cannot be opened as `FjallJournal`.
- `Error::RunNotFound` — run exists in header index but has zero journal events.
- `Error::JournalRead` — journal read failure (events, header, snapshot) after open.

## Contract Signatures

```rust
// crates/velvet_ballastics/src/commands_ai_context.rs

/// Handle the `ai-context` CLI subcommand.
/// Emits a bounded, redacted AI context packet for the given run.
pub(crate) fn handle(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode;

pub(crate) fn redacted_slot_value(
    slot: vb_core::SlotIdx,
    value: Option<&Vec<u8>>,
    snapshot: Option<&vb_storage::RunSnapshot>,
) -> Value;

pub(crate) fn suggested_ai_commands(
    run_id: &str,
    db: &std::path::Path,
    status: RunStatus,
) -> Vec<String>;
```

All fallible internal helpers return `Result<T, E>` or propagate `ExitCode`. No `unwrap`, `expect`, or `panic` in the public surface.

## Non-goals

- Runtime state inspection (live shard memory) — this command reads only persisted journal and snapshots.
- Modifying run state, canceling runs, or triggering actions.
- Serving AI context over HTTP or a daemon — output is stdout JSON only.
