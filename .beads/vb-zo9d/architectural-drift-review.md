bead_id: vb-zo9d
bead_title: cli/storage: Report journal trim eligibility in doctor
phase: 13
updated_at: 2026-05-09T22:00:00Z

# Architectural Drift Review

## Line Count Check

| File | Total Lines | My Diff Lines | Pre-existing? |
|---|---|---|---|
| `crates/vb_storage/src/trimming.rs` | 1457 | +31 | Yes (was already 1400+) |
| `crates/vb_storage/src/error.rs` | 341 | +29 | Yes (was already 300+) |
| `crates/velvet_ballastics/src/main.rs` | 4061 | +31 | Yes (was already 4000+) |

**Assessment:** My changes added only ~30 lines to each pre-existing file. The files were
already well over the 300-line threshold before this bead. Splitting these files is
out of scope for bead vb-zo9d and would be a separate refactoring bead.

## DDD Review (Scott Wlaschin)

### Primitive Obsession
- `TrimDiagnostic` uses `u64` for counts but wraps them in a named struct with
documentation. This is acceptable — the struct name provides domain context.
- `TrimPolicy` uses `u32` for `retain_last_n_terminal` — appropriately typed for
the domain (number of runs to retain).

### Parse, Don't Validate
- `latest_durable_snapshot_seq` returns `Option<EventSeq>` — parsed at the boundary,
not validated as raw bytes ✓
- `count_trimmable_events` parses key bytes into `EventSeq` with `try_into` ✓

### Types as Documentation
- No boolean parameters in new public APIs ✓
- `TrimEligibility` and `TrimBlocker` are expressive enums ✓

### Workflows as Explicit State Transitions
- Diagnostic workflow: open journal → scan headers → check snapshot → check retention
→ count events → aggregate → report. Each step is explicit and sequential ✓

### Newtypes
- Uses existing `RunId`, `EventSeq` newtypes from `vb_core` ✓
- No raw primitives in public API signatures ✓

## Functional Core / Imperative Shell

- `trim_eligibility_diagnostic`: Imperative shell (I/O scan) with pure logic inside
  (eligibility classification, aggregate counting) ✓
- `count_trimmable_events`: Imperative shell (fjall iteration) with pure comparison
  inside (seq < safe_point) ✓
- `cmd_doctor`: Imperative shell (CLI I/O) calling into the storage diagnostic ✓

## Refactoring Needed?

No. The new code is clean, well-typed, and follows DDD principles. The pre-existing
file sizes are a codebase-wide concern, not introduced by this bead.

## Decision

STATUS: APPROVED

No architectural drift introduced by bead vb-zo9d. The new types (`TrimEligibility`,
`TrimBlocker`, `TrimDiagnostic`) are well-designed domain types that follow Scott
Wlaschin's DDD principles. No refactoring required.
