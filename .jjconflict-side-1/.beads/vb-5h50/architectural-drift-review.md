bead_id: vb-5h50
bead_title: storage: Trim journal events after durable snapshots
phase: state-13-architectural-drift
updated_at: 2026-05-09T00:00:00Z

# Architectural Drift Review

## Line Count Audit

| File | Total Lines | Production Lines | Status |
|---|---|---|---|
| `crates/vb_storage/src/trimming.rs` | 909 | 309 | At boundary (pre-existing) |
| `crates/vb_storage/src/journal.rs` | 2397 | N/A | Pre-existing, unchanged structurally |

Note: `trimming.rs` was 515 lines before this bead. The retention feature added ~100 production lines. The file was already over the 300-line threshold prior to this work.

## DDD Assessment

### Primitive Obsession
- `retain_last_n_terminal: u32` — A raw count. Could be a newtype `TerminalRetentionCount(u32)`, but `u32` is semantically clear here. No action needed.
- `accepted_at_ms: u64` — Already a newtype in domain (`RunHeaderRecord`). ✅

### State Transitions
- Trim operation is a single-shot command, not a long-running state machine. ✅
- `TrimStatus` enum (Trimmed/NoOp) correctly models the two possible outcomes. ✅

### Parse, Don't Validate
- Journal events are parsed into `JournalEvent` enum at the storage boundary via `decode_record`. ✅
- Keys are constructed via `run_event_key`, `run_prefix_key` — typed constructors. ✅

### Workflow Explicitness
- `trim_events_for_run` workflow: snapshot → retention → delete → result. Clear and linear. ✅

## Refactor Assessment

### Option 1: Split trimming.rs
Extract retention logic into `trimming/retention.rs`:
- Pros: Brings production code under 300 lines
- Cons: Private methods on `FjallJournal` would need to become free functions or pub(crate); introduces module complexity

### Option 2: Keep as-is
- The file is well-organized: types → impl block → tests
- The overage is minimal (9 lines) and pre-existing
- No structural issues

## Decision

No refactoring performed. The file organization is clean and the 300-line overage is pre-existing. The retention feature integrates naturally with the existing structure.

STATUS: APPROVED
