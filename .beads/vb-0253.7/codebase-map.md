# vb-0253.7 Codebase Map: CLI Lifecycle Tracker Event-Applied

## Bead
- **ID**: vb-0253.7
- **Title**: cli: Make lifecycle tracker event-applied
- **Phase**: 1 (Explore and scope)
- **Source**: /home/lewis/src/velvet-ballistics (read-only checkout)
- **Isolated Workspace**: /home/lewis/src/femdation-vb-0253-7

## Executive Summary

The `vb_cli` crate's `lifecycle.rs` module maintains an in-memory `RunStateTracker` (via `static TRACKER: LazyLock<Mutex<RunStateTracker>>`) that tracks run lifecycle states separately from the journal. This creates a sync risk: the in-memory state can diverge from the authoritative journal. The "event-applied" refactoring will make state derivation always come from journal events.

## Primary Crate: vb_cli

**Path**: `crates/vb_cli/`

### Core File: lifecycle.rs (582 lines)

**Problem**: `RunStateTracker` uses a global static mutex with in-memory `HashMap<RunId, LifecycleState>`. State is set AFTER journal writes, creating a window where in-memory state can diverge from persisted journal.

**Key Structures**:
```rust
// In-memory tracker (line 40-43)
struct RunStateTracker {
    states: std::collections::HashMap<RunId, LifecycleState>,
}

// Global static (line 62-63)
static TRACKER: std::sync::LazyLock<std::sync::Mutex<RunStateTracker>> = ...

// Tracker accessors (line 66-79, 82-95)
fn with_tracker<F, T>(run: RunId, f: F) -> Result<T, CoreError>
fn with_tracker_mut<F, T>(run: RunId, f: F) -> Result<T, CoreError>
```

**Public API** (lines 97-429):
| Function | Purpose | Journal Write | In-Memory Update |
|----------|---------|--------------|------------------|
| `cancel(run, journal)` | Cancel a run | `JournalEvent::RunCancelled` | `set_state(Cancelled)` |
| `resume(run, journal)` | Resume a run | `JournalEvent::RunResumed` | `set_state(Active)` |
| `retry(run, journal)` | Retry a failed run | `JournalEvent::RunRetried` | `set_state(Active)` |
| `answer(run, answer, journal)` | Answer a waiting run | `JournalEvent::RunAnswered` | `set_state(Completed)` |
| `replay(journal)` | Derive states from journal | None | Rebuilds from events |

**State Machine** (from `vb_core::workflow::LifecycleState`, lines 1787-1800):
- `Pending` → `Active` → `WaitingAnswer` ↔ `Cancelled`
- `Active` → `Failed` → `Active` (via retry)
- `WaitingAnswer` → `Completed` (via answer)

**Event-to-State Derivation** (lines 502-526):
```rust
fn derive_lifecycle_state_from_events(events: &[JournalEvent]) -> LifecycleState {
    // Last event determines state:
    // RunCancelled → Cancelled
    // RunResumed/RunRetried/RunAccepted/RunAdmission → Active
    // RunAnswered/RunFinished → Completed
    // RunFailedEvent → Failed
    // WaitScheduledEvent/AskScheduledEvent/AskAnsweredEvent → WaitingAnswer
    // ActionFailedEvent → Failed
}
```

**Test Helpers** (lines 541-581):
- `set_lifecycle_state(run, state)` - bypasses journal, TEST ONLY
- `reset_tracker()` - clears in-memory state, TEST ONLY
- `create_run_header(journal, run)` - TEST ONLY

### Related Files in vb_cli

| File | Purpose |
|------|---------|
| `args.rs` (1700+ lines) | CLI argument parsing; defines `Command::Cancel/Retry/Resume/Answer` |
| `storage.rs` (295 lines) | Journal operations: `cmd_inspect`, `cmd_events`, `cmd_replay` |
| `run.rs` (214 lines) | `cmd_validate`, `cmd_compile`, `cmd_run` |
| `main.rs` (28 lines) | Entry point, calls `app_impl::run_from_env()` |
| `lib.rs` (5 lines) | `pub mod lifecycle; pub mod naming_scan;` |

### Commands Using Lifecycle (from args.rs)

```rust
Command::Retry { run_id, db, output }     // lines 120-124
Command::Resume { run_id, db, output }     // lines 125-129
Command::Answer { run_id, step, value_file, db, output } // lines 143-149
Command::Cancel { run_id, db, reason, output } // lines 176-181
```

## Dependencies

### vb_cli → vb_core
- `vb_core::errors::CoreError` - Error types including lifecycle errors
- `vb_core::ids::RunId`, `SlotIdx`, `SymbolId` - ID types
- `vb_core::workflow::{LifecycleCommand, LifecycleState, RunState, check_lifecycle_transition}`
- `vb_core::value::ConstValue`
- `vb_core::CompiledWorkflow`, `WorkflowParts`, `WorkflowDigest`

### vb_cli → vb_storage
- `vb_storage::EventSeq` - Per-run event sequence numbers
- `vb_storage::FjallJournal` - Journal storage
- `vb_storage::JournalEvent` - All event variants

## Scope for "Event-Applied" Refactoring

### Files to Modify
1. **`crates/vb_cli/src/lifecycle.rs`** - Core refactoring
2. Possibly **`crates/vb_cli/src/commands_incident.rs`** - If incident report building needs changes

### Key Changes Required

1. **Remove `RunStateTracker` in-memory state**:
   - Remove `struct RunStateTracker { states: HashMap<RunId, LifecycleState> }`
   - Remove `static TRACKER: LazyLock<Mutex<RunStateTracker>>`
   - Remove `with_tracker()` and `with_tracker_mut()` helpers

2. **Make state derivation event-applied**:
   - `cancel()` → query journal for current state via `derive_lifecycle_state_from_events()` before writing
   - `resume()` → same
   - `retry()` → same
   - `answer()` → same
   - Remove all `with_tracker_mut()` calls that set state

3. **Transition validation remains event-applied**:
   - `check_lifecycle_transition()` already exists in `vb_core`
   - Call `journal.events_for_run(run)` to get current state before validating transitions

4. **Test infrastructure adaptation**:
   - `test_helpers` module may need revision or removal
   - Tests must write journal events, not bypass with direct state set

### Public API Surface (No Change Expected)
```rust
pub type LifecycleResult<T> = Result<T, CoreError>;

pub fn cancel(run: RunId, journal: &FjallJournal) -> LifecycleResult<()>
pub fn resume(run: RunId, journal: &FjallJournal) -> LifecycleResult<()>
pub fn retry(run: RunId, journal: &FjallJournal) -> LifecycleResult<()>
pub fn answer(run: RunId, answer: String, journal: &FjallJournal) -> LifecycleResult<()>
pub fn replay(journal: &FjallJournal) -> LifecycleResult<Vec<RunState>>
```

## Risk Tags

| Risk | Severity | Description |
|------|----------|-------------|
| `state-sync` | HIGH | In-memory tracker can diverge from journal; must always query events |
| `static-mutex` | MEDIUM | Global `LazyLock<Mutex<...>>` is a concurrency bottleneck and failure point |
| `test-only-bypass` | MEDIUM | `test_helpers::set_lifecycle_state` bypasses journal - tests may not reflect real behavior |
| `journal-read-latency` | LOW | Reading full event sequence on each lifecycle command adds latency |
| `backwards-compat` | LOW | Public API unchanged, but internal behavior different |

## Required Verifier Modes

| Mode | Justification |
|------|----------------|
| `KANI` | Bounded model checking for state transition correctness |
| `MIRI` | Detects undefined behavior in unsafe blocks (none currently, but watch for new ones) |
| `LOOM` | If concurrency testing needed for the static mutex pattern |

## Contracts

### Lifecycle State Machine Contract
```
check_lifecycle_transition(s, cmd) = true  ⟹  transition s --cmd--> s' is valid
derive_lifecycle_state_from_events(events).last() = s  ⟹  events体现s
```

### Error Contract
All lifecycle functions return `CoreError` variants:
- `LifecycleInvalidTransition` - transition not allowed from current state
- `LifecycleDuplicateRequest` - command already applied
- `LifecycleStaleRequest` - run already in terminal state
- `LifecycleStorageUnavailable` - tracker lock failed
- `JournalWriteFailure` - journal append failed

## File Manifest

```
crates/vb_cli/
├── src/
│   ├── lifecycle.rs          # PRIMARY: 582 lines, RunStateTracker, lifecycle API
│   ├── lib.rs                # 5 lines, module exports
│   ├── args.rs               # 1700+ lines, CLI argument parsing
│   ├── storage.rs            # 295 lines, journal commands
│   ├── commands_incident.rs   # 115 lines, incident report building
│   ├── run.rs                # 214 lines, run/compile commands
│   ├── main.rs               # 28 lines, entry point
│   └── ...
├── Cargo.toml                # dependencies on vb_core, vb_storage
└── ...
```

## Discovery Notes

- The `replay()` function (lines 441-489) already implements correct event-applied behavior
- `derive_lifecycle_state_from_events()` (lines 502-526) is the canonical state derivation
- `test_helpers` module explicitly marks functions as TEST ONLY with bypass warnings
- `check_lifecycle_transition()` in `vb_core` handles all valid transition logic