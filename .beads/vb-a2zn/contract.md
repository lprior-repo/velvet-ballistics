# Contract Specification — vb-a2zn

## Context

- **Bead**: vb-a2zn — durability: normalize absent-run query outcomes
- **Domain terms**: JournalError, NoEvents, FjallJournal, events_for_run, absent run, CLI exit code consistency, read-only queries (inspect, events, replay, trace, retry, resume, diff)
- **Assumptions**: Fjall journal provides ordered durable events; all read-only CLI commands must agree on exit codes and typed errors for absent runs; `events_for_run` is the single source of truth for run-existence queries
- **Open questions**: None

---

## Problem Statement

`FjallJournal::events_for_run` returns `Ok(Vec::new())` for unknown or empty runs. This creates divergent CLI behavior:

| Command | Current behavior for absent run | Exit code |
|---|---|---|
| `events` | Ok branch, empty vec → prints "no events found" | 2 (ValidationFailed) |
| `inspect` | Ok branch, empty vec → prints "no events found" | 2 (ValidationFailed) |
| `replay` | `recover_full_journal` has own empty check → `NoRecoveryData` | 5 (StorageError) |
| `trace` | `read_journal_events` returns Ok([]) → prints "no events found" | 0 (Success) |
| `retry` | `read_journal_events` returns Ok([]) → empty check → prints "not found" | 5 (StorageError) |
| `resume` | `read_journal_events` returns Ok([]) → empty check → prints "not found" | 5 (StorageError) |
| `diff` | `events_for_run` returns Ok([]) → no empty check → diffs empty vectors | 0 (Success) |

Three distinct exit codes (0, 2, 5) for the same semantic condition (run has no events). This is a contract violation.

---

## Root Cause

`events_for_run` → `events_for_run_from` iterates `snap.prefix(&self.events, run_prefix_key(run))`. When no prefix matches, the for-loop produces zero iterations and the function returns `Ok(replay)` where `replay` is `Vec::new()`. No distinction between "run exists but has no events" and "run does not exist."

---

## Preconditions

- **PRE-001**: `events_for_run(run)` — run is a valid `RunId` (parseable u64); no assumption that run exists in journal
- **PRE-002**: `events_for_run_from(run, start_seq)` — run is a valid `RunId`; `start_seq >= EventSeq::new(0)`
- **PRE-003**: `recover_full_journal(journal, run, tracker, _, _)` — calls `events_for_run` first; if events empty, returns `RecoveryError::NoRecoveryData`
- **PRE-004**: All read-only CLI commands (inspect, events, replay, trace, retry, resume, diff) — `--db` points to a valid Fjall journal path
- **PRE-005**: `NoEvents` error is semantically distinct from `ProcessLockHeld` (writer lock) and all other `JournalError` variants

---

## Postconditions

- **POST-001**: `events_for_run(run)` returns `Err(JournalError::NoEvents { run })` iff no journal events exist for `run`; never returns `Ok([])`
- **POST-002**: `events_for_run_from(run, start_seq)` returns `Err(JournalError::NoEvents { run })` iff no journal events for `run` have `seq >= start_seq`; never returns `Ok([])`
- **POST-003**: `JournalError::NoEvents` variant exists in the `JournalError` enum with field `run: RunId`
- **POST-004**: `CliExitCode::from(JournalError::NoEvents { .. })` maps to `CliExitCode::ValidationFailed` (exit code 2) — consistent with all other absent-run handling
- **POST-005**: All read-only CLI commands (inspect, events, replay, trace, retry, resume, diff) return exit code 2 for absent runs — exit code consistency invariant
- **POST-006**: `read_journal_events` helper propagates `NoEvents` through `events_for_run` Err branch (no empty-vec check needed)
- **POST-007**: `cmd_trace` — when `NoEvents` is returned, prints error and returns `CliExitCode::StorageError` (via `read_journal_events` Err branch)
- **POST-008**: `cmd_diff` — when either run has `NoEvents`, prints error and returns `CliExitCode::StorageError`
- **POST-009**: `recover_full_journal` — `events_for_run` Err propagates via `?` to caller; `recover_full_journal` still retains its own empty check as a belt-and-suspenders defense (the `?` from `events_for_run` would short-circuit before reaching the empty check, making the empty check dead code — implementer must remove it)
- **POST-010**: `cmd_events` — removes redundant empty-vec check after `events_for_run`; all paths go through Err branch
- **POST-011**: `cmd_inspect` — removes redundant empty-vec check after `events_for_run`; all paths go through Err branch
- **POST-012**: `cmd_retry` and `cmd_resume` — remove redundant empty-vec checks after `read_journal_events`; `read_journal_events` returns Err on `NoEvents`

---

## Invariants

- **INV-001**: `events_for_run` NEVER returns `Ok(Vec::new())` — empty result is always an error
- **INV-002**: `NoEvents` is the ONLY path to empty events from `events_for_run` and `events_for_run_from`; no `Ok([])` exists in the contract surface
- **INV-003**: All read-only CLI commands return the SAME exit code (2 = ValidationFailed) when the target run has no events
- **INV-004**: `ProcessLockHeld` is orthogonal to `NoEvents` — lock check happens during `FjallJournal::open`, `NoEvents` happens during `events_for_run`; they cannot be conflated
- **INV-005**: `NoEvents` discriminant must not overlap with any existing `JournalError` discriminant (the enum has no explicit discriminants, so no overlap is automatic, but this invariant documents intent for future enum changes)
- **INV-006**: `NoEvents` is `#[non_exhaustive]` compatible — adding fields to the variant is a non-breaking change
- **INV-007**: `From<JournalError> for CliExitCode` maps `NoEvents` to `ValidationFailed` — no special-casing needed if the blanket impl covers it

---

## Error Taxonomy

| Error Variant | When Raised | Semantic |
|---|---|---|
| `JournalError::NoEvents { run }` | `events_for_run` or `events_for_run_from` finds zero events for the requested run | Absent run — run not found or has zero events |
| `JournalError::ProcessLockHeld` | `FjallJournal::open` fails — writer holds exclusive lock | Writer lock — temporary, not an absent-run condition |
| `RecoveryError::NoRecoveryData { run }` | `recover_full_journal` calls `events_for_run`, gets empty vec (belt-and-suspenders) | Absent run — recovery-layer typed error |
| `JournalError::Fjall` | Low-level Fjall I/O failure | Storage infrastructure error |
| `JournalError::Encode` | Postcard encoding failure during append | Serialization error |

---

## Type Model

### New variant on `JournalError`

```rust
/// No journal events found for the requested run.
///
/// This is distinct from storage infrastructure errors:
/// the run simply has zero events in the journal.
#[error("no events found for run {run:?}")]
NoEvents {
    /// Run identifier that has no events.
    run: RunId,
},
```

### Updated signature

```rust
impl FjallJournal {
    /// Returns events for a run in contiguous per-run sequence order.
    ///
    /// # Errors
    /// Returns `JournalError::NoEvents` if no events exist for `run`.
    pub fn events_for_run(&self, run: RunId) -> Result<Vec<JournalEvent>, JournalError>;

    /// Returns events for a run starting from a given sequence.
    ///
    /// # Errors
    /// Returns `JournalError::NoEvents` if no events exist for `run` with `seq >= start_seq`.
    pub(crate) fn events_for_run_from(
        &self,
        run: RunId,
        start_seq: EventSeq,
    ) -> Result<Vec<JournalEvent>, JournalError>;
}
```

### CLI exit code contract

| CLI command | Absent run exit code |
|---|---|
| `inspect` | 2 (ValidationFailed) |
| `events` | 2 (ValidationFailed) |
| `replay` | 2 (ValidationFailed) |
| `trace` | 2 (ValidationFailed) |
| `retry` | 2 (ValidationFailed) |
| `resume` | 2 (ValidationFailed) |
| `diff` (either run absent) | 2 (ValidationFailed) |

---

## Contract Signatures

```rust
// crates/vb_storage/src/error/mod.rs — new variant
pub enum JournalError {
    // ... existing variants ...
    /// No journal events found for the requested run.
    #[error("no events found for run {run:?}")]
    NoEvents { run: RunId },
}

// crates/vb_storage/src/journal/replay.rs — updated signatures
impl FjallJournal {
    pub fn events_for_run(&self, run: RunId) -> Result<Vec<JournalEvent>, JournalError>;
    pub(crate) fn events_for_run_from(
        &self,
        run: RunId,
        start_seq: EventSeq,
    ) -> Result<Vec<JournalEvent>, JournalError>;
}

// crates/vb_cli/src/exit_code.rs — From impl covers NoEvents (blanket impl)
// NoEvents → StorageError via blanket From<JournalError>
// BUT: We need NoEvents → ValidationFailed for exit-code consistency.
// Solution: NoEvents maps to ValidationFailed via explicit impl.
```

---

## Exit Code Mapping

```rust
impl From<JournalError> for CliExitCode {
    fn from(err: JournalError) -> Self {
        match err {
            JournalError::NoEvents { .. } => CliExitCode::ValidationFailed,
            _ => CliExitCode::StorageError,
        }
    }
}
```

---

## Verification Layers

- **Verus-owned kernel**: `events_for_run` and `events_for_run_from` return `Err(NoEvents)` iff prefix scan yields zero events; no `Ok([])` path exists
- **Static-scan**: `JournalError::NoEvents` variant exists; `From<JournalError>` maps it correctly; no `Ok([])` in events_for_run code path
- **Test (BDD)**: All seven read commands (inspect, events, replay, trace, retry, resume, diff) return exit code 2 for absent runs

---

## Non-goals

- Modifying `recover_full_journal` behavior (it already handles empty via its own check; the empty check becomes dead code and should be removed)
- Modifying `recover_snapshot_plus_tail` (different code path, takes events as parameter)
- Adding `NoEvents` to `RecoveryError` (RecoveryError already has `NoRecoveryData` for this case)
- Changing `ProcessLockHeld` handling (orthogonal concern — writer lock check happens at `open`, not at `events_for_run`)
- Modifying write commands (run, submit, run-compiled, answer, cancel, resume-action)
- IPC server behavior changes

---

## Implementation Notes for Follow-On Agent

1. **Add `NoEvents` to `JournalError`** in `crates/vb_storage/src/error/mod.rs` — after `ProcessLockIo`, before `Trim`. Include `run: RunId` field. Add `#[error("no events found for run {run:?}")]` attribute.

2. **Update `events_for_run_from`** in `crates/vb_storage/src/journal/replay.rs` — after the for-loop, check `if replay.is_empty() { return Err(JournalError::NoEvents { run }); }`.

3. **Update `From<JournalError> for CliExitCode`** in `crates/vb_cli/src/exit_code.rs` — match on `NoEvents { .. }` → `ValidationFailed`. All other variants → `StorageError`.

4. **Simplify `cmd_events`** in `crates/vb_cli/src/app_impl.rs` — remove the `if events.is_empty()` check after `Ok(events)`. The Err branch handles it now.

5. **Simplify `cmd_inspect`** — same removal of empty-vec check.

6. **Simplify `cmd_retry` and `cmd_resume`** — remove empty-vec check after `read_journal_events`.

7. **Remove dead code in `recover_full_journal`** — the `if events.is_empty()` check is now unreachable because `events_for_run` returns `Err(NoEvents)` before the `?` propagates. Remove it.

8. **Verify `cmd_diff`** — already uses `events_for_run` directly with Err handling. No changes needed after step 2 (Err branch catches NoEvents).

9. **Verify `cmd_trace`** — uses `read_journal_events` which calls `events_for_run`. Err branch handles it. No changes needed.
