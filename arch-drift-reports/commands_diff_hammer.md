# Architectural Drift Report: `commands_diff.rs`

**File**: `crates/vb_cli/src/commands_diff.rs`
**Total Lines**: 964
**Status**: REFACTOR REQUIRED (exceeds 300-line limit by 321%)

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 964 | 300 | **VIOLATION** |
| Production code | ~357 | 300 | VIOLATION |
| Test code | ~607 | — | N/A |

**Required split**:
- `diff_core.rs` — pure diff computation (line 1–357)
- `diff_tests.rs` — all tests (line 359–964)

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Raw `u16` for Step and Slot Indices

**Location**: `collect_step_outcomes` (line 319), `collect_slot_values` (line 342)

```rust
// VIOLATION: HashMap<u16, String> — raw u16 for step index
pub fn collect_step_outcomes(events: &[JournalEvent]) -> HashMap<u16, String> {
    let mut outcomes = HashMap::new();
```

```rust
// VIOLATION: HashMap<u16, String> — raw u16 for slot index  
pub fn collect_slot_values(events: &[JournalEvent]) -> HashMap<u16, String> {
    let mut slots = HashMap::new();
```

**Required fix**: Use `StepIdx` and `SlotIdx` newtypes (already imported from `vb_core`).

### 2.2 Stringly-Typed Diff Kinds

**Location**: `compute_diff` (line 21–130)

All diff kinds are string literals:

```rust
"only_in_a"        // line 34
"only_in_b"        // line 41
"changed"          // line 49
"step_missing_in_b" // line 65
"step_outcome_differs" // line 73
"slot_missing_in_b" // line 98
"slot_value_differs" // line 106
"step_missing_in_a" // line 85
"slot_missing_in_a" // line 118
```

**Required fix**: Define `enum DiffKind { OnlyInA, OnlyInB, Changed, ... }` and serialize to string only at I/O boundary.

### 2.3 Raw `usize` for Event Counts

**Location**: `DiffResult` struct (line 11–18)

```rust
pub struct DiffResult {
    pub events_a: usize,  // VIOLATION
    pub events_b: usize, // VIOLATION
    pub diffs: Vec<serde_json::Value>, // VIOLATION: JSON Value is primitive
}
```

**Required fix**: `EventCount(u64)` newtype wrapper, `DiffEntry` struct instead of `serde_json::Value`.

### 2.4 `Vec<serde_json::Value>` for Structured Diff Output

**Location**: Line 17, 25

```rust
pub diffs: Vec<serde_json::Value>, // VIOLATION: loses type safety
```

**Required fix**: Define `struct DiffEntry { kind: DiffKind, index: Option<u64>, ... }` enum.

### 2.5 Stringly-Typed Outcome Formatting

**Location**: Lines 324, 327, 332

```rust
outcomes.insert(step.get(), format!("succeeded(output={})", output.get()));
outcomes.insert(step.get(), format!("failed(action={})", action.get()));
outcomes.insert(step.get(), format!("action_completed(action={})", action.get()));
```

**Required fix**: Define `enum StepOutcome { Succeeded { output: SlotIdx }, Failed { action: ActionId }, ActionCompleted { action: ActionId } }` as a proper value object.

---

## 3. DDD STRUCTURAL VIOLATIONS

### 3.1 Workflow Blur — I/O Concern in Pure Logic

`diff_event_summary` (line 133–216) mixes domain logic with JSON serialization:

```rust
pub fn diff_event_summary(event: &JournalEvent) -> serde_json::Value {
    // Every branch returns serde_json::json!(...)
}
```

**DDD Principle violated**: "Parse, don't validate" / domain logic should not produce presentation types.

**Required fix**: Create `enum DiffEventSummary` domain type, convert to JSON at I/O boundary.

### 3.2 Missing Value Objects

| Raw Type | Location | Should Be |
|----------|----------|-----------|
| `u16` step index | `HashMap<u16, String>` keys | `StepIdx` |
| `u16` slot index | `HashMap<u16, String>` keys | `SlotIdx` |
| `String` outcome | HashMap values | `StepOutcome` enum |
| `String` display | HashMap values | `SlotDisplay` enum |
| `&'static str` event name | `event_name` return | `EventKind` enum |

### 3.3 `events_differ` Has Signature Coupling to Raw Primitives

**Location**: Line 244–316

The function returns `bool` and compares only semantically significant fields. This is correct but incomplete — the "significance" decision is implicit in pattern matching rather than modeled as explicit `SignificantFields` or similar.

---

## 4. COMMAND RESPONSIBILITIES (Single Responsibility Principle)

The file contains **four distinct responsibilities** that must be separated:

| Responsibility | Functions | Lines |
|----------------|-----------|-------|
| Event stream diff computation | `compute_diff` | 21–130 |
| Event to summary conversion | `diff_event_summary`, `event_name` | 133–241 |
| Semantic event comparison | `events_differ` | 244–316 |
| Event aggregation | `collect_step_outcomes`, `collect_slot_values` | 319–357 |
| Test suite | All `#[test]` modules | 359–964 |

---

## 5. REFACTOR PRESCRIPTION

### 5.1 File Split

```
vb_cli/src/
├── diff/
│   ├── mod.rs
│   ├── diff_core.rs      # compute_diff, events_differ (~150 lines)
│   ├── diff_types.rs     # DiffKind, DiffEntry, StepOutcome, SlotDisplay (~100 lines)  
│   ├── diff_aggregation.rs  # collect_step_outcomes, collect_slot_values (~50 lines)
│   ├── diff_summary.rs   # diff_event_summary, event_name (~60 lines)
│   └── diff_tests.rs     # All tests (~607 lines)
```

### 5.2 Newtypes Required

```rust
// In diff_types.rs
pub enum DiffKind {
    OnlyInA,
    OnlyInB,
    Changed,
    StepMissingInB,
    StepMissingInA,
    StepOutcomeDiffers,
    SlotMissingInB,
    SlotMissingInA,
    SlotValueDiffers,
}

pub enum StepOutcome {
    Succeeded { output: SlotIdx },
    Failed { action: ActionId },
    ActionCompleted { action: ActionId },
}

pub enum SlotDisplay {
    Decoded(String),
    None,
    Bytes(usize),
}

pub struct DiffEntry {
    pub kind: DiffKind,
    pub index: Option<u64>,
    pub step: Option<StepIdx>,
    pub event_a: Option<DiffEventSummary>,
    pub event_b: Option<DiffEventSummary>,
    pub outcome_a: Option<StepOutcome>,
    pub outcome_b: Option<StepOutcome>,
    pub value_a: Option<SlotDisplay>,
    pub value_b: Option<SlotDisplay>,
}

pub enum DiffEventSummary {
    RunAccepted { seq: u64 },
    RunAdmission { seq: u64, policy: String },
    StepStarted { seq: u64, step: StepIdx },
    // ... variants
}
```

### 5.3 Aggregator Fixes

```rust
// Before (primitive obsession)
pub fn collect_step_outcomes(events: &[JournalEvent]) -> HashMap<u16, String>

// After (DDD compliant)
pub fn collect_step_outcomes(events: &[JournalEvent]) -> HashMap<StepIdx, StepOutcome>
```

---

## 6. VERDICT

| Check | Status |
|-------|--------|
| Under 300 lines | ❌ FAIL (964 lines) |
| No primitive obsession | ❌ FAIL (u16, String, usize throughout) |
| Value objects for domain concepts | ❌ FAIL |
| Parse don't validate | ❌ FAIL (stringly-typed diff kinds) |
| Explicit state transitions | ⚠️ PARTIAL (events_differ is correct intent) |
| Single responsibility | ❌ FAIL (5 responsibilities co-mingled) |

**REQUIRED ACTIONS**:
1. Split file into `diff_core.rs`, `diff_types.rs`, `diff_aggregation.rs`, `diff_summary.rs`, `diff_tests.rs`
2. Replace all `HashMap<u16, String>` with `HashMap<StepIdx, StepOutcome>` / `HashMap<SlotIdx, SlotDisplay>`
3. Replace all string diff kinds with `enum DiffKind`
4. Replace `Vec<serde_json::Value>` with `Vec<DiffEntry>`
5. Update `mod.rs` to expose new module structure

---

*Report generated: architectural-drift enforcer*
*Workspace: arch-drift-hammer (JJ)*
