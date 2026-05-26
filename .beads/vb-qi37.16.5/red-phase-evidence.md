# Red-Phase Evidence: vb-qi37.16.5

## Bead Information

- **Bead ID**: vb-qi37.16.5
- **Title**: cli/runtime: Add lifecycle integration evidence
- **Phase**: State 5 (red-phase test writing)
- **Date**: 2026-05-11

## Evidence Summary

The integration tests in `crates/velvet_ballistics/tests/lifecycle_integration.rs` are **correctly failing** because the lifecycle command surface does not exist yet in `velvet_ballistics`. This document records the exact compilation errors and the missing implementation components.

## Compilation Evidence

### Command

```bash
cargo test --package velvet_ballistics --test lifecycle_integration 2>&1
```

### Primary Errors

#### Error 1: Missing Module `velvet_ballistics::lifecycle`

```
error[E0433]: cannot find module or crate `velvet_ballistics` in this scope
  --> crates/velvet_ballistics/tests/lifecycle_integration.rs:79:18
   |
79 |     let result = velvet_ballistics::lifecycle::cancel(run, &journal);
   |                  ^^^^^^^^^^^^^^^^^ use of unresolved module or unlinked crate `velvet_ballistics`
```

**Root Cause**: The `velvet_ballistics::lifecycle` module does not exist. The lifecycle commands (cancel, resume, retry, answer) must be exposed as a public API in `velvet_ballistics`.

**Required Implementation**:
- Create `crates/velvet_ballistics/src/lifecycle.rs` module
- Expose public functions: `cancel`, `resume`, `retry`, `answer`, `replay`
- These should delegate to `vb_runtime::shard` handlers or provide a new CLI-facing API

#### Error 2: Missing Error Variants in `vb_core::errors::CoreError`

The tests reference error variants that don't exist yet:

- `vb_core::errors::CoreError::LifecycleInvalidTransition`
- `vb_core::errors::CoreError::LifecycleDuplicateRequest`
- `vb_core::errors::CoreError::LifecycleStaleRequest`
- `vb_core::errors::CoreError::ReplayCorruption`
- `vb_core::errors::CoreError::StorageUnavailable`
- `vb_core::errors::CoreError::JournalWriteFailure`

**Root Cause**: The `vb_core::errors::CoreError` enum does not have lifecycle-specific error variants.

**Required Implementation**:
- Add variants to `crates/vb_core/src/errors.rs`:
  ```rust
  LifecycleInvalidTransition {
      code: String,
      context: String,
      timestamp: vb_core::DiagnosticTimestamp,
      bead_id: RunId,
      command: LifecycleCommand,
  },
  LifecycleDuplicateRequest { ... },
  LifecycleStaleRequest { ... },
  // etc.
  ```
- Add `LifecycleCommand` enum to `crates/vb_core/src/workflow.rs`

## Missing Implementation Components

### 1. Lifecycle Module in velvet_ballistics

**Location**: `crates/velvet_ballistics/src/lifecycle.rs` (to be created)

**Functions Required** (per contract.md and test-plan.md):

```rust
// Cancel command
pub fn cancel(bead_id: RunId, journal: &FjallJournal) -> Result<(), CoreError>;

// Resume command
pub fn resume(bead_id: RunId, journal: &FjallJournal) -> Result<(), CoreError>;

// Retry command
pub fn retry(bead_id: RunId, journal: &FjallJournal) -> Result<(), CoreError>;

// Answer command
pub fn answer(bead_id: RunId, answer: String, journal: &FjallJournal) -> Result<(), CoreError>;

// Replay command
pub fn replay(journal: &FjallJournal) -> Result<Vec<BeadState>, CoreError>;
```

### 2. Error Variants in CoreError

**Location**: `crates/vb_core/src/errors.rs`

**Variants Required**:
- `LifecycleInvalidTransition` - POST-003
- `LifecycleDuplicateRequest` - POST-004
- `LifecycleStaleRequest` - POST-005
- `ReplayCorruption` - journal corruption during replay
- `StorageUnavailable` - PRE-001
- `JournalWriteFailure` - storage I/O error

### 3. Structured Diagnostic Fields

Each error variant must include:
- `code: String` - error code (e.g., "E_INVALID_TRANSITION")
- `context: String` - human-readable description
- `timestamp: vb_core::DiagnosticTimestamp` - ISO-8601 UTC timestamp
- `bead_id: RunId` - target bead
- `command: LifecycleCommand` - the command that failed

### 4. LifecycleState Enum

**Location**: `crates/vb_core/src/workflow.rs`

States required:
- `Pending` - initial state
- `Active` - running
- `WaitingAnswer` - suspended awaiting external answer
- `Cancelled` - manually cancelled
- `Failed` - failed
- `Completed` - terminal success

### 5. LifecycleCommand Enum

**Location**: `crates/vb_core/src/workflow.rs` (new)

Commands:
- `Cancel`
- `Resume`
- `Retry`
- `Answer`

## Test Coverage

The red-phase integration tests cover:

### Group A: Happy Path (5 tests)
- `cancel_succeeds_when_bead_is_active`
- `cancel_succeeds_when_bead_is_waiting_answer`
- `resume_succeeds_when_bead_is_cancelled`
- `retry_succeeds_when_bead_is_failed`
- `answer_succeeds_when_bead_is_waiting_answer`

### Group B: Invalid Transitions (18 tests)
- Each command from each invalid prior state

### Group C: Duplicate Requests (4 tests)
- Duplicate cancel, resume, retry, answer

### Group D: Stale Requests (4 tests)
- Stale cancel, resume, retry, answer

### Group E: Restart/Replay (4 tests)
- Empty journal replay
- Full journal fidelity
- Malformed event
- Missing event

### Group F: Storage I/O Errors (2 tests)
- Storage unavailable (PRE-001)
- Journal write failure

### Group G: Structured Diagnostics (3 tests)
- InvalidTransition, DuplicateRequest, StaleRequest diagnostics

### Group H: State Transition Graph (2 tests)
- Valid edges exist
- No self-loops

### Integration: Exactly-One-Event (1 test)
- POST-001 verification

**Total**: 43 integration tests

## Contract Alignment

| Contract Clause | Tests | Status |
|----------------|-------|--------|
| PRE-001 | Group F | Red-phase - missing storage unavailable handling |
| PRE-002 | Group B | Red-phase - missing command validation |
| POST-001 | Group A + Integration | Red-phase - missing lifecycle commands |
| POST-002 | Group E | Red-phase - missing replay |
| POST-003 | Group B | Red-phase - missing InvalidTransition error |
| POST-004 | Group C | Red-phase - missing DuplicateRequest error |
| POST-005 | Group D | Red-phase - missing StaleRequest error |
| INV-001 | Group H | Red-phase - missing LifecycleState enum |
| INV-002 | Group C | Red-phase - append-only verification |
| INV-003 | Group H | Red-phase - valid transitions |
| INV-004 | Group E | Red-phase - replay fidelity |

## Next Steps

To move from red-phase to green-phase:

1. **Create `crates/vb_core/src/errors.rs` additions**: Add lifecycle error variants with structured diagnostics
2. **Create `crates/vb_core/src/workflow.rs` additions**: Add `LifecycleState` and `LifecycleCommand` enums
3. **Create `crates/velvet_ballistics/src/lifecycle.rs`**: Implement the lifecycle command API
4. **Implement error conversions**: Add `From<lifecycle::Error>` for appropriate error types
5. **Implement journal integration**: Connect lifecycle commands to vb_storage journal

## Files Modified

- `crates/velvet_ballistics/tests/lifecycle_integration.rs` (created - red-phase tests)
- `.beads/vb-qi37.16.5/red-phase-evidence.md` (this file - evidence documentation)

## Files Requiring Implementation

1. `crates/vb_core/src/errors.rs` - Add lifecycle error variants
2. `crates/vb_core/src/workflow.rs` - Add LifecycleState, LifecycleCommand
3. `crates/velvet_ballistics/src/lifecycle.rs` - Implement lifecycle command API
4. `crates/velvet_ballistics/src/main.rs` or `args.rs` - Wire CLI to lifecycle module
5. `crates/vb_storage/src/journal.rs` - May need additional journal event types