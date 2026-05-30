# Architectural Drift Report: gate_12_14_15.rs

**File:** `crates/vb_validate/src/gate_12_14_15.rs`
**Line Count:** 443 (VIOLATION: exceeds 300 line limit by 47.7%)
**Classification:** CRITICAL DRIFT — Immediate refactoring required

---

## Executive Summary

This file triples as a validation module for three distinct workflow gates (12, 14, 15). It violates the fundamental architectural rule of **single responsibility** and compounds the violation with severe **primitive obsession** throughout. The file must be split into three separate modules before any further work proceeds.

---

## 1. Gate Responsibility Map

| Gate | Function | Responsibility |
|------|----------|----------------|
| **12** | `validate_gate_12_action_contract_completeness` | Verifies every `Do` node's action has a contract, and no contract is orphaned |
| **14** | `validate_gate_14_slot_type_consistency` | Verifies each slot receives only one type of constant value |
| **15** | `validate_gate_15_determinism_proof` | Verifies no non-deterministic node follows another non-deterministic node |

---

## 2. Line Count Violation

**Rule:** Files must not exceed 300 lines.

**Actual:** 443 lines

**Breakdown:**
- Gate 12 logic: ~42 lines (lines 11-53)
- Gate 14 logic: ~34 lines (lines 55-89)
- Gate 15 logic: ~28 lines (lines 102-130)
- Helper `const_value_discriminant`: ~9 lines (lines 91-100)
- Test module: **310 lines** (lines 132-443)

**Verdict:** The test code alone (310 lines) exceeds the 300-line limit for the entire file. Tests must be moved to separate files under `crates/vb_validate/tests/` or a sibling test module.

---

## 3. Primitive Obsession Violations (DDD Anti-Pattern)

### 3.1 Raw `u16` for Action Identifiers

**Lines 15, 32-33:**
```rust
let mut do_action_ids: Vec<u16> = Vec::new();
// ...
if !do_action_ids.contains(&action_val) {
    do_action_ids.push(action_val);
}
```

**Problem:** `action_val` is already `u16` from `ActionId::get()`. Should be using `ActionId` or `ActionIdSet` (a `HashSet<ActionId>`).

**Refactor:** Replace `Vec<u16>` with `BTreeSet<ActionId>` or `ActionIdSet`.

---

### 3.2 Raw `u8` Slot Kind Discriminant

**Line 60:**
```rust
let mut slot_const_kind: Vec<u8> = vec![0; slot_count];
```

**Problem:** Using raw `u8` to represent type kind. This is the textbook definition of **primitive obsession**.

**Lines 91-99 — `const_value_discriminant` function:**
```rust
fn const_value_discriminant(value: &vb_core::value::ConstValue) -> u8 {
    match value {
        vb_core::value::ConstValue::Null => 1,
        vb_core::value::ConstValue::Bool(_) => 2,
        vb_core::value::ConstValue::I64(_) => 3,
        vb_core::value::ConstValue::F64(_) => 4,
        vb_core::value::ConstValue::Symbol(_) => 5,
        _ => 0,
    }
}
```

**Problem:** This is a manual enum-to-int encoding. The `ConstValue` type already has a discriminant — use it directly or create a proper `SlotKind` enum.

**Refactor:**
```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum SlotKind {
    Null,
    Bool,
    I64,
    F64,
    Symbol,
    Unknown,
}

impl From<&ConstValue> for SlotKind {
    fn from(v: &ConstValue) -> Self {
        match v {
            ConstValue::Null => SlotKind::Null,
            ConstValue::Bool(_) => SlotKind::Bool,
            ConstValue::I64(_) => SlotKind::I64,
            ConstValue::F64(_) => SlotKind::F64,
            ConstValue::Symbol(_) => SlotKind::Symbol,
            _ => SlotKind::Unknown,
        }
    }
}
```

---

### 3.3 Raw `usize` Conversions

**Lines 28, 48:**
```rust
action_id: usize::from(action_val),
// ...
action_id: usize::from(cid),
```

**Problem:** `usize::from(u16)` is a lossy widening cast for error reporting. If `ActionId` is a newtype wrapper, it should implement `Display` or `Debug` for error context.

---

### 3.4 Raw Index Arithmetic

**Lines 63-64, 70-71:**
```rust
let cidx = value.as_usize();
if cidx >= parts.constants.len() {
// ...
let su = slot.as_usize();
if su < slot_count {
```

**Problem:** `ConstIdx::as_usize()` and `SlotIdx::as_usize()` reveal that these are wrappers around raw integers. The validation logic then does manual bounds checking against raw `usize` values.

---

## 4. Scott Wlaschin DDD Violations

### 4.1 No Value Objects

The file treats primitive types (`u16`, `u8`, `usize`) as domain concepts. According to Wlaschin's DDD, we should have:
- `ActionId` (already exists in `vb_core`) — but we extract raw `u16` instead of using the type
- `SlotKind` — does not exist, creating a phantom type gap
- `SlotTypeFingerprint` — suggested for Gate 14's concept of "what type lives in this slot"

### 4.2 No Domain-Aligned Error Types

`ValidationError::ActionContractMissing { action_id: usize, node_index }` uses raw `usize` instead of `ActionId` and `NodeIndex`. Error messages should preserve domain types.

### 4.3 O(n²) Algorithm in Gate 12

**Lines 17-25:**
```rust
for (node_index, node) in parts.nodes.iter().enumerate() {
    if let CompiledNodeKind::Do { action, .. } = &node.kind {
        let action_val = action.get();
        let mut found = false;
        for contract in action_contracts {  // <-- NESTED LOOP
            if contract.id.get() == action_val {
                found = true;
                break;
            }
        }
```

**Problem:** Linear search through all contracts for each Do node. With `d` Do nodes and `c` contracts, this is O(d × c).

**Refactor:** Build a `HashSet<ActionId>` from contracts once, then `contains()` in O(1).

---

### 4.4 Imperative Accumulator Pattern

**Lines 15, 32-34:**
```rust
let mut do_action_ids: Vec<u16> = Vec::new();
// ...
if !do_action_ids.contains(&action_val) {
    do_action_ids.push(action_val);
}
```

**Problem:** Manual deduplication logic using `Vec` instead of `Set`.

**Refactor:** Use `BTreeSet<ActionId>` or `ActionIdSet`.

---

## 5. File Structure Violation

This single file contains:
- 3 public gate validation functions
- 1 private helper function
- 1 test module with **17 test cases**

**Rule violation:** Files should be cohesive units. This file has 4 distinct cohesion axes:
1. Gate 12 validation
2. Gate 14 validation
3. Gate 15 validation
4. Shared test helpers

---

## 6. Required Refactoring

### Minimum viable fix (do not exceed 2 weeks of change):
1. **Split into 3 modules:** `gate_12.rs`, `gate_14.rs`, `gate_15.rs`
2. **Move tests** to `tests/gate_12_rs_tests.rs`, etc.
3. **Replace `Vec<u16>` with `BTreeSet<ActionId>`** in gate 12
4. **Create `SlotKind` enum** replacing the `u8` discriminant
5. **Replace `slot_const_kind: Vec<u8>` with `Vec<Option<SlotKind>>`**

### Ideal fix (recommended):
1. All of the above
2. Add `ActionIdSet` value object
3. Add domain errors that preserve `ActionId` type
4. Replace linear search with HashSet lookup

---

## 7. Evidence

```
File: gate_12_14_15.rs
Lines: 443
Limit: 300
Violation: +143 lines (+47.7%)

Primitive Obsession Count: 6+ instances
DDD Violations: 4+ categories
Cohesion Score: FAIL (4 concerns in 1 file)
```

---

*Report generated by arch-drift-hammer*
*Severity: CRITICAL*
*Action Required: Mandatory refactor before further commits*
