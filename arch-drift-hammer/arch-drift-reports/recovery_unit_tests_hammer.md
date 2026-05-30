# ARCH-DRIFT HAMMER REPORT
## Target: `recovery_unit_tests.rs`
## Severity: CRITICAL

---

## EXECUTIVE SUMMARY

| Metric | Value | Status |
|--------|-------|--------|
| File lines | **1092** | ❌ VIOLATION (>300) |
| Primitive obsession violations | **18+** | ❌ CRITICAL |
| Test groups | **13** | ⚠️ SHOULD SPLIT |
| Lines per responsibility | ~84 avg | ⚠️ OVERSIZED |

---

## VIOLATION 1: FILE SIZE — 1092 LINES (LIMIT: 300)

**VERDICT**: This file is **3.6x over budget**. It combines 13 logically distinct test responsibilities into a single 1092-line monolith.

### Required Split

| Test Group | Suggested File | Est. Lines |
|------------|---------------|------------|
| `recovery_error_*` | `recovery_error_tests.rs` | ~150 |
| `recovery_terminal_state_*` | `terminal_state_tests.rs` | ~60 |
| `recovery_runtime_summary_construction` | `summary_tests.rs` | ~80 |
| `unsupported_recovery_state_*` | `unsupported_state_tests.rs` | ~70 |
| `summarize_recovery_events_*` | `summarize_events_tests.rs` | ~160 |
| `recover_run_admission_from_events_*` | `admission_tests.rs` | ~70 |
| `recover_runtime_frame_seed_from_events_*` | `frame_seed_tests.rs` | ~120 |
| `apply_summary_event_*` | `apply_summary_tests.rs` | ~170 |
| `action_replay_tracker_*` | `tracker_tests.rs` | ~60 |
| `replay_events_*` | `replay_events_tests.rs` | ~70 |
| `is_terminal_event_*` | `terminal_predicate_tests.rs` | ~80 |
| `extract_terminal_*` | `extract_terminal_tests.rs` | ~120 |
| `recovery_error_match_covers_all_variants` | `recovery_error_exhaustiveness.rs` | ~30 |

---

## VIOLATION 2: PRIMITIVE OBSESSION — RECOVERYRUNTIMESUMMARY

**Location**: Lines 197-227, 231-246, 622-770

### Problem: Struct Uses Raw Integers

```rust
// ❌ VIOLATION: All these are raw i64/u32 instead of domain types
RecoveryRuntimeSummary {
    run: RunId::new(1),
    first_seq: EventSeq::new(0),
    last_seq: EventSeq::new(10),
    workflow: Some(sample_digest(9)),
    steps_started: 5,           // raw integer
    steps_succeeded: 4,         // raw integer
    actions_scheduled: 3,       // raw integer
    actions_resolved: 3,        // raw integer
    suspensions: 2,            // raw integer
    slots_written: 6,           // raw integer
    terminal: Some(RecoveryTerminalState::Finished { result: SlotIdx::new(1) }),
}
```

### Required Refactor: Introduce Value Types

```rust
// ✅ REQUIRED: Newtypes for counters
pub struct StepCount(u32);
pub struct ActionCount(u32);
pub struct SuspensionCount(u32);
pub struct SlotWriteCount(u32);

pub struct RecoveryRuntimeSummary {
    run: RunId,
    first_seq: EventSeq,
    last_seq: EventSeq,
    workflow: Option<WorkflowDigest>,
    steps_started: StepCount,
    steps_succeeded: StepCount,
    actions_scheduled: ActionCount,
    actions_resolved: ActionCount,
    suspensions: SuspensionCount,
    slots_written: SlotWriteCount,
    terminal: Option<RecoveryTerminalState>,
}
```

### Evidence: Repeated Throughout File

- Line 202: `steps_started: 5`
- Line 203: `steps_succeeded: 4`
- Line 204: `actions_scheduled: 3`
- Line 205: `actions_resolved: 3`
- Line 206: `suspensions: 2`
- Line 207: `slots_written: 6`
- Line 217: `assert_eq!(summary.steps_started, 5);` // magic number
- Line 218: `assert_eq!(summary.steps_succeeded, 4);` // magic number
- Lines 236-241: Same pattern repeated in `recovery_runtime_summary_with_no_terminal`

**COUNT**: 18+ raw integer fields across summary construction and assertions.

---

## VIOLATION 3: PRIMITIVE OBSESSION — TERMINAL STATE STRINGS

**Location**: Lines 130-143

### Problem: Raw Strings for Typed States

```rust
// ❌ VIOLATION: "Finished" and "Cancelled" are raw strings
let expected = "Finished".to_string();
let found = "Cancelled".to_string();
let err = RecoveryError::TerminalStateMismatch {
    expected: expected.clone(),
    found: found.clone(),
};
```

### Required Refactor

```rust
// ✅ REQUIRED: Use typed enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalStateLabel {
    Finished,
    Cancelled,
    Failed,
}

let expected = TerminalStateLabel::Finished;
let found = TerminalStateLabel::Cancelled;
```

---

## VIOLATION 4: PRIMITIVE OBSESSION — ERROR DETAIL STRING

**Location**: Line 98

```rust
// ❌ VIOLATION: Raw String for domain error detail
let detail = "step ordering violation".to_owned();
```

### Required Refactor

```rust
// ✅ REQUIRED: Typed error detail
pub struct ReplayDivergenceDetail(String);

impl ReplayDivergenceDetail {
    pub fn step_ordering_violation() -> Self { ... }
    pub fn missing_event() -> Self { ... }
}
```

---

## VIOLATION 5: ATTEMPT INDEXING — RAW `u32`

**Location**: Lines 317, 318, 319, 323, 400, 401, 404, 405, 412, 413, 419, 420, 427, etc.

### Problem: `attempt: 1` Is Untyped

Every event construction uses raw `u32` for attempt numbering:

```rust
JournalEvent::StepStarted {
    run,
    seq: EventSeq::new(1),
    step: StepIdx::new(0),
    attempt: 1,  // ❌ raw integer
},
```

### Required Refactor

```rust
// ✅ REQUIRED: Typed attempt index
pub struct Attempt(u32);

JournalEvent::StepStarted {
    run,
    seq: EventSeq::new(1),
    step: StepIdx::new(0),
    attempt: Attempt::new(1),
},
```

---

## RESPONSIBILITY MAP

| Responsibility | Lines | Tests | Primitive Violations |
|----------------|-------|-------|---------------------|
| RecoveryError variants | 25-153 | 12 | 0 |
| RecoveryTerminalState | 159-185 | 5 | 0 |
| RecoveryRuntimeSummary | 191-246 | 2 | 7 |
| UnsupportedRecoveryState | 252-297 | 5 | 0 |
| summarize_recovery_events | 303-447 | 4 | 0 |
| recover_run_admission | 453-504 | 3 | 0 |
| recover_runtime_frame_seed | 510-614 | 5 | 0 |
| apply_summary_event | 620-771 | 6 | 9 |
| ActionReplayTracker | 777-813 | 4 | 0 |
| replay_events | 819-876 | 3 | 0 |
| is_terminal_event | 882-958 | 6 | 0 |
| extract_terminal | 964-1066 | 5 | 0 |
| Exhaustiveness check | 1068-1091 | 1 | 0 |

---

## DDD ASSESSMENT

### What Works ✓
- `RecoveryError` is a proper tagged union (algebraic datatype)
- `RecoveryTerminalState` variants are well-typed
- `ActionReplayTracker` encapsulates tracking logic
- Event types (`JournalEvent::*`) are proper enums

### What Violates DDD ✗
- `RecoveryRuntimeSummary` is an **anemic data bag** — all operations are in test code, not on the type
- Counters (`steps_started`, `suspensions`, etc.) should be **value objects**, not raw integers
- No **domain invariants** enforced: nothing prevents `steps_succeeded > steps_started`
- Error details are **untyped strings** instead of **error code types**

---

## HAMMER ACTIONS REQUIRED

### 1. SPLIT FILE (Mandatory)

Create 13 separate test files under `crates/vb_storage/src/recovery/tests/`:

```
recovery/
├── tests/
│   ├── recovery_error_tests.rs
│   ├── terminal_state_tests.rs
│   ├── summary_tests.rs
│   ├── unsupported_state_tests.rs
│   ├── summarize_events_tests.rs
│   ├── admission_tests.rs
│   ├── frame_seed_tests.rs
│   ├── apply_summary_tests.rs
│   ├── tracker_tests.rs
│   ├── replay_events_tests.rs
│   ├── terminal_predicate_tests.rs
│   ├── extract_terminal_tests.rs
│   └── recovery_error_exhaustiveness.rs
└── recovery_unit_tests.rs  (delete)
```

### 2. INTRODUCE COUNTER TYPES (Mandatory)

In `vb_storage/src/recovery/types.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StepCount(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ActionCount(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SuspensionCount(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SlotWriteCount(u32);
```

### 3. INTRODUCE TERMINAL STATE LABEL TYPE (Mandatory)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalStateLabel {
    Finished,
    Cancelled,
    Failed,
}
```

### 4. INTRODUCE ATTEMPT TYPE (Mandatory)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Attempt(u32);
```

---

## SCORE

| Criterion | Score | Max |
|-----------|-------|-----|
| File size compliance | 0 | 30 |
| Primitive obsession eradication | 5 | 30 |
| DDD cohesion | 15 | 25 |
| Test responsibility isolation | 5 | 15 |
| **TOTAL** | **25** | **100** |

**VERDICT**: ❌ **REJECTED** — File exceeds size limit by 3.6x; 18+ primitive obsession violations; anemic domain model.

---

*Report generated by arch-drift-hammer*
*Workspace: velvet-ballistics*
*Date: 2026-05-29*
