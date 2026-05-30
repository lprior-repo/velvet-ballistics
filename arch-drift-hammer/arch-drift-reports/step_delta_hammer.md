# Architectural Drift Report: `step_delta.rs`

**File:** `crates/vb_cli/src/properties/step_delta.rs`
**Line Count:** 502 (exceeds 300-line limit by 202 lines)
**Severity:** CRITICAL

---

## Executive Summary

This file is a **501-line property test module** that computes diffs between slot/taint/state arrays. It violates the <300 line rule AND multiple Scott Wlaschin DDD principles including primitive obsession, type-driven design, and code duplication.

---

## CRITICAL VIOLATION 1: File Size (>300 Lines)

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 502 | 300 | 🔴 OVER |
| Test code | ~450 | - | - |
| Helper structs | 4 | - | - |
| Duplicate delta functions | 3 | 0 | 🔴 OVER |

---

## CRITICAL VIOLATION 2: Primitive Obsession (DDD Anti-Pattern)

### Problem A: Raw `u16` for Domain Indices

```rust
// Lines 19-23 — SlotDelta uses raw u16
struct SlotDelta {
    slot: u16,              // 🔴 SHOULD BE SlotIdx
    before: Option<SlotValue>,
    after: Option<SlotValue>,
}

// Lines 26-30 — TaintDelta uses raw u16
struct TaintDelta {
    slot: u16,              // 🔴 SHOULD BE SlotIdx
    before: Taint,
    after: Taint,
}

// Lines 32-37 — StateDelta uses raw u16
struct StateDelta {
    step: u16,              // 🔴 SHOULD BE StepIdx
    before: StepState,
    after: StepState,
}
```

**The file imports `SlotIdx` and `StepIdx` from `vb_core::ids` (line 7) but then IGNORES them**, using raw `u16` instead. This is textbook primitive obsession — the domain types exist but are bypassed.

### Problem B: No Value Object Extraction

The file defines 4 local delta structs that should be **first-class domain value objects** in `vb_core`:

| Local Struct | Should Be | Location |
|--------------|-----------|----------|
| `SlotDelta` | `SlotDelta` (value object) | `vb_core::frame` |
| `TaintDelta` | `TaintDelta` (value object) | `vb_core::frame` |
| `StateDelta` | `StateDelta` (value object) | `vb_core::frame` |
| `PcDelta` | `PcDelta` (value object) | `vb_core::frame` |

---

## CRITICAL VIOLATION 3: Code Duplication (DRY Violation)

### Three Identical Functions (Lines 45-116):

```rust
fn compute_slot_deltas(...) -> Vec<SlotDelta>   // Pattern: enumerate + filter_map
fn compute_taint_deltas(...) -> Vec<TaintDelta>  // Pattern: enumerate + filter_map  
fn compute_state_deltas(...) -> Vec<StateDelta>  // Pattern: enumerate + filter_map
```

These three functions share **100% identical structure**:
1. Take two slices
2. Find min length
3. Zip with enumerate
4. Filter where before != after
5. Collect into delta struct

**Should be ONE generic function:**
```rust
fn compute_deltas<T, U, V>(before: &[U], after: &[U], f: F) -> Vec<T>
where F: Fn(usize, &U, &U) -> Option<T>
```

---

## VIOLATION 4: Missing Domain Abstraction

### What Should Exist in `vb_core::frame`:

```rust
// vb_core/src/frame/delta.rs (NEW FILE — VALUE OBJECT)
use crate::ids::{SlotIdx, StepIdx};
use crate::value::SlotValue;
use crate::value::Taint;
use crate::frame::StepState;

/// A delta between two slot value arrays
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotDelta {
    pub slot: SlotIdx,
    pub before: Option<SlotValue>,
    pub after: Option<SlotValue>,
}

/// A delta between two taint arrays  
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaintDelta {
    pub slot: SlotIdx,
    pub before: Taint,
    pub after: Taint,
}

/// A delta between two step state arrays
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDelta {
    pub step: StepIdx,
    pub before: StepState,
    pub after: StepState,
}

/// A delta between two program counters
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcDelta {
    pub before: StepIdx,
    pub after: StepIdx,
}

/// Generic diff between two arrays
pub fn compute_deltas<T, U, F>(before: &[U], after: &[U], mut f: F) -> Vec<T>
where
    F: FnMut(usize, &U, &U) -> Option<T>,
{
    let count = before.len().min(after.len());
    before[..count]
        .iter()
        .zip(after[..count].iter())
        .enumerate()
        .filter_map(|(i, (b, a))| f(i, b, a))
        .collect()
}
```

### This File Should Import From Core:

```rust
use vb_core::frame::delta::{SlotDelta, TaintDelta, StateDelta, PcDelta};
use vb_core::frame::delta::compute_deltas;
```

---

## VIOLATION 5: Three-Test Pattern Repetition

The file has **6 near-identical test blocks** that all follow the same pattern:

1. `slot_deltas_only_include_changed_slots` (lines 189-219)
2. `taint_deltas_only_include_changed_taints` (lines 222-251)
3. `state_deltas_only_include_changed_states` (lines 253-283)
4. `slot_deltas_bounds` (lines 299-315)
5. `taint_deltas_bounds` (lines 318-334)
6. `state_deltas_bounds` (lines 336-353)

These should be **table-driven proptest tests** or a single parameterized test.

---

## Action Items

| Priority | Action | Target |
|----------|--------|--------|
| P0 | Extract `SlotDelta`, `TaintDelta`, `StateDelta`, `PcDelta` as value objects to `vb_core::frame::delta` | `vb_core/` |
| P0 | Replace raw `u16` with `SlotIdx`/`StepIdx` in all delta types | `vb_core::frame::delta` |
| P0 | Create generic `compute_deltas()` function | `vb_core::frame::delta` |
| P1 | Rewrite this file to import from `vb_core::frame::delta` | `step_delta.rs` |
| P1 | Consolidate 6 repetitive tests into parameterized variants | `step_delta.rs` |
| P2 | Target: reduce file to <300 lines | `step_delta.rs` |

---

## Conclusion

**This file is a 502-line blob of test infrastructure that should be:**
1. A 150-line thin wrapper importing domain value objects
2. The actual delta types should live in `vb_core::frame::delta` as first-class citizens

**The primitive obsession is particularly egregious** — the file imports `SlotIdx` and `StepIdx` but uses raw `u16` throughout, completely defeating the type system's ability to catch slot/step confusion bugs.
