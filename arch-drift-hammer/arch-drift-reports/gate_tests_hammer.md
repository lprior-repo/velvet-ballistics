# ARCHITECTURAL DRIFT HAMMER REPORT
## Target: `/home/lewis/src/velvet-ballistics/crates/vb_validate/src/gate_tests.rs`

**File Status:** 981 LINES — **CATASTROPHIC VIOLATION** of 300-line rule

---

## EXECUTIVE SUMMARY

```
╔═══════════════════════════════════════════════════════════════════╗
║  CRITICAL: File is 327% OVER the 300-line hard limit               ║
║  VIOLATIONS: Primitive Obsession, Monster Module, No DDD cohesion ║
╚═══════════════════════════════════════════════════════════════════╝
```

---

## 1. FILE SIZE VIOLATION (PRIMARY)

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 981 | 300 | **FAIL** (+327%) |
| Gates per file | 6 | 1 | **FAIL** |
| Test functions | 47 | ~20 max | **FAIL** |

**Verdict:** This file MUST be split into at minimum 4 separate files.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Raw Primitives in Helper Constructors (Lines 12-29)

```rust
// VICTIM: make_parts helper
fn make_parts(
    nodes: Vec<CompiledNode>,
    slot_count: u16,      // ← PRIMITIVE OBSESSION
    symbols_count: u32,   // ← PRIMITIVE OBSESSION
) -> WorkflowParts
```

**Problem:** `slot_count` and `symbols_count` are raw `u16`/`u32` instead of domain types.

**Fix:** Introduce `SlotCount(u16)` and `SymbolCount(u32)` wrapper types.

### 2.2 Raw u16 Magic Numbers Throughout (Lines 271-351)

```rust
// Line 273: raw u16 magic number
output: Some(SlotIdx::new(99)), // out of range for slot_count=1

// Lines 288, 323, 342: more raw u16 abuse
copy_node(0, 50, 0); // source=50 out of range
fields: Box::new([(SymbolId::new(1), SlotIdx::new(99))]),
items: Box::new([SlotIdx::new(50)]),
```

**Problem:** Hardcoded magic numbers `99`, `50` scattered everywhere.

### 2.3 Raw u32 MAX Constants (Lines 216, 239)

```rust
// Lines 216 and 239 - DUPLICATE TEST
path: Box::new([PathSegment::Index(u32::MAX)]),
```

**Problem:** Sentinel value `u32::MAX` used directly instead of a named constant like `SENTINEL_INDEX`.

### 2.4 Raw Integer in Struct Fields (Lines 383, 418, 441)

```rust
// Line 383 - raw u16 in domain struct
limit: 10,  // ← should be MaxIterations(10) or similar

// Lines 418, 441 - raw StepIdx with magic numbers
body: StepIdx::new(99),  // out of range
done: StepIdx::new(99),  // out of range
```

---

## 3. DUPLICATE TEST VIOLATIONS

### 3.1 `gate_08_rejects_sentinel_index_segment` vs `gate_08_rejects_max_value_index_segment`

| Aspect | Test A (L212-222) | Test B (L235-244) |
|--------|-------------------|-------------------|
| Name | `gate_08_rejects_sentinel_index_segment` | `gate_08_rejects_max_value_index_segment` |
| Line | 212 | 235 |
| Body | **IDENTICAL** | **IDENTICAL** |
| Check | `u32::MAX` | `u32::MAX` |
| Error | `AccessorPathInvalid` | `AccessorPathInvalid` |

**Verdict:** These are copy-pasted clones. Test B must be deleted.

### 3.2 Redundant Boundary Test (Lines 316-318)

```rust
#[test]
fn gate_09_accepts_slot_at_boundary_slot_count_minus_one() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_09_slot_references(&parts), Ok(()));
}
```

**Problem:** This test is functionally identical to `gate_09_accepts_valid_slot_references` (L264-267). The boundary is already tested by construction of `finish_node(0, 0)`.

---

## 4. MONOLITHIC MODULE VIOLATION

### 4.1 Six Gates in One File

```
gate_tests.rs
├── Gate 7 tests (lines 75-174)      ~100 lines
├── Gate 8 tests (lines 182-255)      ~74 lines
├── Gate 9 tests (lines 263-357)      ~95 lines
├── Gate 11 tests (lines 365-610)     ~246 lines
├── Gate 13 tests (lines 618-913)     ~296 lines
└── Exhaustiveness test (lines 916-981) ~66 lines
```

**Problem:** Each gate test suite should be in its own file:
- `gate_07_stack_tests.rs`
- `gate_08_accessor_tests.rs`
- `gate_09_slots_tests.rs`
- `gate_11_loop_tests.rs`
- `gate_13_cycles_tests.rs`

### 4.2 Inline Helper Modules (Lines 72-73, 180, 261, 363, 616)

```rust
// These `use` statements are gate-specific but embedded in monolithic file
use crate::gate_07_stack::validate_gate_07_expression_stack_depth;
use crate::gate_07_stack::compute_stack_depth;
```

**Problem:** Tests should be in their own files with local `use` statements.

---

## 5. EXHAUSTIVENESS TEST BLOAT (Lines 916-981)

```rust
#[test]
fn validation_error_match_covers_all_variants() {
    fn _exhaustive_match(e: &ValidationError) -> &'static str {
        match e {
            ValidationError::DuplicateKey => "duplicate_key",
            // ... 58 more arms
        }
    }
    let _ = _exhaustive_match;
}
```

**Problems:**
1. 65 lines of repetitive match arms — should be a proc-macro or derived
2. This is a **compile-time** check wearing a test costume
3. Lives in wrong location — should be in a separate `validation_error_tests.rs`

---

## 6. SCOTT WLASCHIN DDD VIOLATIONS

### 6.1 No Value Objects for Validation Boundaries

| Primitive | Should Be |
|-----------|-----------|
| `u16` slot count | `SlotCount(u16)` |
| `u32` symbol count | `SymbolCount(u32)` |
| `u32::MAX` index sentinel | `IndexSentinel` or `SENTINEL_INDEX` |
| `u16` step index | `StepIndex(StepIdx)` |
| `u16` loop limit | `IterationLimit(u16)` |

### 6.2 Tests Should Use Domain Builders

```rust
// CURRENT (primitive obsession):
let node = CompiledNode {
    id: StepIdx::new(0),
    output: Some(SlotIdx::new(99)), // raw magic
    ...
};

// SHOULD BE:
let node = test_node()
    .with_output_slot(Slot::out_of_range(99))  // Domain semantic
    .build();
```

---

## 7. REQUIRED REFACTORING

### 7.1 File Splitting Plan

```
crates/vb_validate/src/
├── gate_tests.rs                    # KEEP: Shared helpers only (~80 lines)
├── gate_07_stack_tests.rs           # NEW: Gate 7 tests (~100 lines)
├── gate_08_accessor_tests.rs        # NEW: Gate 8 tests (~75 lines)
├── gate_09_slots_tests.rs           # NEW: Gate 9 tests (~95 lines)
├── gate_11_loop_tests.rs            # NEW: Gate 11 tests (~246 lines)
├── gate_13_cycles_tests.rs          # NEW: Gate 13 tests (~296 lines)
└── validation_error_exhaustiveness.rs  # NEW: Match test (~30 lines)
```

### 7.2 New Value Object Types Required

```rust
// In vb_core domain - new types needed:
pub struct SlotCount(pub u16);
pub struct SymbolCount(pub u32);
pub struct IterationLimit(pub u16);
pub const SENTINEL_INDEX: u32 = u32::MAX;
```

### 7.3 Delete These Tests

1. `gate_08_rejects_max_value_index_segment` (L235-244) — duplicate of L212-222
2. `gate_09_accepts_slot_at_boundary_slot_count_minus_one` (L316-318) — redundant

---

## 8. EVIDENCE COMMANDS

```bash
# Count lines
wc -l crates/vb_validate/src/gate_tests.rs
# Expected: < 300

# Find magic numbers
rg 'SlotIdx::new(9[0-9])' crates/vb_validate/src/gate_tests.rs

# Find duplicate tests
rg 'u32::MAX' crates/vb_validate/src/gate_tests.rs
```

---

## VERDICT

```
╔══════════════════════════════════════════════════════════════════╗
║  ARCHITECTURAL DRIFT: CATASTROPHIC                              ║
║  - 981 lines vs 300 line limit (327% OVER)                     ║
║  - Primitive obsession on every test helper                     ║
║  - Duplicate tests eating lines                                 ║
║  - 6 gates crammed into one monster module                      ║
║  - Exhaustiveness test in wrong location                        ║
║                                                                ║
║  REQUIRED ACTIONS:                                             ║
║  1. Split into 6 separate test files                           ║
║  2. Introduce SlotCount, SymbolCount value objects              ║
║  3. Delete 2 duplicate/redundant tests                         ║
║  4. Move exhaustiveness test to separate file                  ║
║  5. Replace magic numbers with domain-typed builders            ║
╚══════════════════════════════════════════════════════════════════╝
```

---

**Generated by:** architectural-drift enforcer
**Target:** `/home/lewis/src/velvet-ballistics/crates/vb_validate/src/gate_tests.rs`
**Date:** 2026-05-29
