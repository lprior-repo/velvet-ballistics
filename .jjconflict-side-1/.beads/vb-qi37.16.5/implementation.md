# Implementation: vb-qi37.16.5 — State 6 Repair (Final)

## Summary

**COMPILATION STATUS: FIXED** — All 23 original compile errors have been resolved. The `lifecycle_integration` test now compiles successfully.

**TEST STATUS: BLOCK_LOCAL** — Tests fail at runtime due to test design issues (state setup assumptions).

## Original Block (per state-6-block.md)

```bash
rtk cargo test --package velvet_ballastics --test lifecycle_integration
```

**Original failures (23 compile errors):**
1. E0433: `velvet_ballastics::lifecycle` module does not exist (binary-only crate)
2. `EventSeq::ZERO` does not exist in `vb_storage/src/events.rs`
3. `JournalEvent::{RunResumed, RunRetried, RunAnswered}` not handled exhaustively

## Changes Made

### 1. `crates/velvet_ballastics/Cargo.toml` (preserved)

Lib target and chrono dependency already present from prior state.

### 2. `crates/velvet_ballastics/src/lib.rs` (preserved)

Library root already exposing lifecycle module.

### 3. `crates/velvet_ballastics/src/lifecycle.rs` (preserved)

Lifecycle command surface already implemented.

### 4. Test File Fixes (THIS REPAIR)

**`crates/velvet_ballastics/tests/lifecycle_integration.rs`:**

- **Line 545**: `s.lifecycle_state` → `s.lifecycle` (RunState field name fix)
- **Lines 615-620**: Rewrote `storage_unavailable` test to handle fact that failed journal open cannot provide a journal handle to pass to cancel()
- **Lines 653-700**: Fixed structured diagnostic assertions to match actual API:
  - `code.is_empty()` → `code.code() != 0` (DiagnosticCode has no `is_empty()`)
  - Removed `timestamp.is_valid()` (DateTime is always valid when constructed)
  - `bead_id` comparisons updated to `Some(run)` (field is `Option<RunId>`)
  - `command` comparisons updated to `Some("cancel")` (field is `Option<&str>`)

### 5. Production Code Fixes (THIS REPAIR)

Added missing `JournalEvent::{RunResumed, RunRetried, RunAnswered}` match arms in:

- **`crates/velvet_ballastics/src/commands_ai_context.rs`** (`event_to_json`):
  ```rust
  vb_storage::JournalEvent::RunResumed { run, timestamp } => ...
  vb_storage::JournalEvent::RunRetried { run, timestamp } => ...
  vb_storage::JournalEvent::RunAnswered { run, slot_idx, answer, timestamp } => ...
  ```

- **`crates/velvet_ballastics/src/vb.rs`** (`print_event` and `event_to_json`):
  Added same three variants with appropriate output formatting.

- **`crates/velvet_ballastics/src/commands_diff.rs`** (`diff_event_summary` and `event_name`):
  Added same three variants with JSON summary and static name strings.

- **`crates/velvet_ballastics/src/commands_journal.rs`** (`trace_one`):
  Added same three variants with TraceEntry construction.

## Verification

```bash
# Compilation: 0 errors (was 23)
rtk cargo test --package velvet_ballastics --test lifecycle_integration
# Result: cargo build: 0 errors, 8 warnings

# Tests: 25 passed, 18 failed (runtime failures, not compile errors)
```

## BLOCK_LOCAL Analysis

**Classification: BLOCK_LOCAL**

**Owner: State 6 repair (this bead)**

**Rerun From: 6**

### Evidence of Test Design Issues

Tests fail at runtime because they assume state transitions persist across tests, but:

1. The global `TRACKER` is reset between test invocations
2. Tests call lifecycle commands on `RunId::new(N)` without setting up initial state
3. Unknown runs default to `LifecycleState::Pending`
4. Most lifecycle commands return `LifecycleInvalidTransition` from `Pending` state

**Example failures:**
```
retry_succeeds_when_bead_is_failed: 
  expected: bead starts in Failed state → retry succeeds
  actual: bead starts in Pending → LifecycleInvalidTransition

resume_returns_stale_request_when_not_in_cancelled_state:
  expected: bead starts in Cancelled → stale request error
  actual: bead starts in Pending → LifecycleInvalidTransition (not StaleRequest)
```

### Root Cause

The test design assumes:
- Runs can be placed into specific initial states (Failed, Cancelled, etc.)
- State persists across multiple test functions
- Each test can issue a command expecting a specific prior state

The actual implementation:
- Unknown runs start in `Pending` state
- No API exists to set run state without journal events
- Global tracker state is not persistent between test functions

### Required Fix (Not In Scope For This Repair)

Tests need either:
1. **State setup functions**: Add helper functions that create runs in specific states via proper lifecycle transitions (e.g., `setup_bead_in_failed_state()`)
2. **Mock storage**: Use a test-only storage adapter that allows direct state manipulation
3. **Test reorganization**: Convert to integration tests that drive runs through full lifecycle before testing recovery

### Contract Conformance

The production code correctly implements the contract:
- `check_lifecycle_transition(Pending, Retry)` → `false` (correct)
- `check_lifecycle_transition(Failed, Retry)` → `true` (correct)
- `get_state(unknown_run)` → `Pending` (correct per INV-001)

The tests are invalid per the contract because they don't set up valid prior state.

## Non-Negotiables Compliance

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `dbg` in production code
- No unchecked indexing, casts, or arithmetic
- Changes are minimal and targeted to fix compilation
- Event/replay semantics preserved: lifecycle events are no-ops in recovery replay