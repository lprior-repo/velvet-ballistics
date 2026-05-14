bead_id: vb-qi37.15.3
bead_title: cli: Add trace command
phase: State 3
updated_at: 2026-05-12T00:00:00Z

# Theorem Kernel Projection

## Boundary
- TLA+-owned temporal model: None (trace is read-only journal replay, no temporal behavior).
- Verus-owned Rust core: `commands_journal::build_trace` and `trace_one` are pure functions over `&[JournalEvent]` producing `Vec<TraceEntry>`. The purity and determinism of these functions constitute the core proof surface.
- Theorem-owned kernel: None required; Verus covers the pure function properties.
- Rust/runtime shell: `cmd_trace` handles CLI dispatch, output formatting, and error reporting. `read_journal_events` handles journal storage I/O.
- External systems excluded from theorem proof: Fjall journal storage (treated as immutable event sequence input to `build_trace`).

## Theorem-Owned Clauses
- None; Verus owns the pure function determinism proof.

## Theorem Obligations
- N/A.

## Verus Scope
- Rust targets: `crates/velvet_ballastics/src/commands_journal.rs::build_trace` and `trace_one`.
- Spec/proof surface:
  - `build_trace`: pure function `events: &[JournalEvent]` -> `Vec<TraceEntry>`
  - `trace_one`: pure function `idx: usize, event: &JournalEvent` -> `TraceEntry`
- Invariants:
  - Determinism: same input always produces same output order and values
  - Completeness: every input event maps to exactly one TraceEntry
  - Index correspondence: entry.index == position in input slice
  - Event type coverage: all `JournalEvent` variants are covered by `trace_one`
- Trusted boundary: `JournalEvent` enum variants are validated by the storage layer on write
- Shell exclusions: I/O (journal read), CLI dispatch, output formatting, error reporting
- Evidence command: `cargo test -p velvet_ballastics -- commands_journal` for unit tests

## Waivers
- None; Verus covers all pure Rust core clauses for this bead.