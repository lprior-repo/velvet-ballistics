# Architectural Drift Report: `vb_validate/src/gates.rs`

**File**: `/home/lewis/src/velvet-ballistics/crates/vb_validate/src/gates.rs`  
**Analysis Date**: 2026-05-29  
**Analyzer**: architectural-drift skill

---

## Executive Summary

| Metric | Value | Status |
|--------|-------|--------|
| Total Lines | **2894** | ❌ EXCEEDS 300-line limit by **2594 lines** |
| DDD Cohesion | **MULTIPLE DOMAIN CONCEPTS** | ❌ FAIL |
| Inline Tests | **1180 lines** (41% of file) | ❌ FAIL |
| Oversized Functions | **9 functions exceed recommended size** | ❌ FAIL |

---

## 1. Line Count Violation

**STATUS: CRITICAL**

- **Actual**: 2894 lines
- **Limit**: 300 lines
- **Overflow**: +2594 lines (864% over limit)

### Line Distribution

| Section | Lines | Percentage |
|---------|-------|------------|
| Production code (gates + helpers) | 1713 | 59% |
| Inline tests (`#[cfg(test)] mod tests`) | 1180 | 41% |
| **Total** | **2894** | **100%** |

---

## 2. DDD Cohesion Analysis

**DDD Smell Detected**: **YES - `#lang/Bounded_Context_Drift`**

The filename `gates.rs` suggests a single concept ("validation gates"), but the file contains **8 distinct domain concepts** that should be separated:

| Gate | Domain Concept | Suggested Module |
|------|---------------|-----------------|
| Gate 7 | Expression Stack Depth | `expr_stack.rs` |
| Gate 8 | Accessor Path Validation | `accessor_path.rs` |
| Gate 9 | Slot Reference Bounds | `slot_bounds.rs` |
| Gate 10 | Node Kind Constraints | `node_constraints.rs` |
| Gate 11 | Loop Body Graph Structure | `loop_graph.rs` |
| Gate 12 | Action Contract Bijection | `action_contracts.rs` |
| Gate 13 | Slot Dependency Cycles | `slot_cycles.rs` |
| Gate 14 | Slot Type Consistency | `slot_types.rs` |
| Gate 15 | Determinism Proof | `determinism.rs` |

**Principle Violation**: Scott Wlaschin DDD says each file should represent a single domain concept. This file violates that by cramming 9 gate concepts (from 9 different workflow validation concerns) into one file.

---

## 3. Violations Catalog

### VIOLATION 1: File Size Exceeded (CRITICAL)
- **Location**: Entire file
- **Lines**: 2894 / 300 allowed
- **Severity**: CRITICAL
- **Remediation**: Split into 9+ separate module files

### VIOLATION 2: Inline Tests Module (MAJOR)
- **Location**: Lines 1714–2894 (`#[cfg(test)] mod tests`)
- **Size**: 1180 lines
- **Content**: 50+ test functions + 4 test helper functions
- **Problem**: Tests should be in `tests/` directory or behind feature flags, not inline in production modules
- **Remediation**: Move to `vb_validate/tests/gates_tests/`

### VIOLATION 3: Oversized Function - `validate_gate_10_node_kind_specific`
- **Location**: Lines 1120–1363
- **Size**: **243 lines**
- **Problem**: Handles 15+ different `CompiledNodeKind` variants in a single match block
- **Severity**: MAJOR
- **Remediation**: Split into per-kind validator functions

### VIOLATION 4: Oversized Function - `validate_gate_11_loop_body_graph`
- **Location**: Lines 416–505
- **Size**: **89 lines**
- **Severity**: MEDIUM
- **Remediation**: Extract loop pairing logic to separate function

### VIOLATION 5: Oversized Function - `node_reads`
- **Location**: Lines 967–1101
- **Size**: **134 lines**
- **Problem**: Duplicates much of `validate_node_slots` logic
- **Severity**: MEDIUM
- **Remediation**: Extract to shared helper or use trait

### VIOLATION 6: Oversized Function - `validate_node_slots`
- **Location**: Lines 244–372
- **Size**: **128 lines**
- **Severity**: MEDIUM
- **Remediation**: Split by `CompiledNodeKind` categories

### VIOLATION 7: Inline Test Helpers Polluting Production Scope
- **Location**: Lines 1721–1772
- **Functions**: `make_parts`, `nop_node`, `finish_node`, `copy_node`
- **Problem**: Test helpers visible to production code (though not exported)
- **Severity**: MINOR
- **Remediation**: Move to test module or `tests/` directory

### VIOLATION 8: Missing Module Separation
- **Problem**: 9 gate functions + 30+ helper functions all in one file
- **Severity**: MAJOR
- **Remediation**: Create `gates/` subdirectory with individual gate modules

### VIOLATION 9: God Function - `append_node_edges`
- **Location**: Lines 894–912
- **Size**: 18 lines but called in tight loop; could accumulate
- **Severity**: MINOR

---

## 4. Specific Line Counts

| Component | Start Line | End Line | Lines |
|-----------|------------|----------|-------|
| Module docs + imports | 1 | 20 | 20 |
| Gate 7 (expr stack) | 26 | 138 | 112 |
| Gate 8 (accessor path) | 144 | 222 | 78 |
| Gate 9 (slot refs) | 228 | 403 | 175 |
| Gate 11 (loop graph) | 416 | 854 | 438 |
| Gate 13 (slot cycles) | 869 | 1101 | 232 |
| Gate 10 (node kind) | 1107 | 1419 | 312 |
| Gate 12 (action contracts) | 1425 | 1573 | 148 |
| Gate 14 (slot types) | 1586 | 1642 | 56 |
| Gate 15 (determinism) | 1665 | 1708 | 43 |
| **Production Subtotal** | | | **1713** |
| Test module | 1714 | 2894 | 1180 |
| **TOTAL** | | | **2894** |

---

## 5. Remediation Priority

| Priority | Action | Effort | Impact |
|----------|--------|--------|--------|
| **P0 (Critical)** | Split file into `gates/` subdirectory with 9 gate modules | High | Enables parallel development, reduces cognitive load |
| **P0 (Critical)** | Move `#[cfg(test)] mod tests` to `vb_validate/tests/` | Medium | Reduces file to ~1713 lines |
| **P1 (High)** | Split `validate_gate_10_node_kind_specific` (243 lines) | Medium | Below 300-line target per module |
| **P1 (High)** | Split `validate_gate_11_loop_body_graph` (89 lines) | Low | Improves maintainability |
| **P2 (Medium)** | Extract `node_reads` (134 lines) as shared utility | Medium | Reduces duplication |
| **P2 (Medium)** | Deduplicate `validate_node_slots` and `node_reads` | Medium | DRY principle |

---

## 6. Recommended File Structure

```
crates/vb_validate/src/
├── lib.rs
├── gates/
│   ├── mod.rs          # Re-exports all gates
│   ├── gate_07_stack.rs   # ~120 lines
│   ├── gate_08_accessor.rs # ~80 lines
│   ├── gate_09_slots.rs    # ~180 lines
│   ├── gate_10_node.rs     # ~150 lines (after split)
│   ├── gate_11_loop.rs     # ~200 lines (after split)
│   ├── gate_12_contract.rs # ~150 lines
│   ├── gate_13_cycles.rs   # ~150 lines
│   ├── gate_14_types.rs    # ~60 lines
│   └── gate_15_determinism.rs # ~50 lines
└── error.rs
```

---

## Conclusion

**STATUS: MUST REFACTOR**

The file `gates.rs` is **864% over the 300-line limit** and violates fundamental DDD cohesion principles. It mixes 9 distinct domain validation concepts in a single file, with inline tests comprising 41% of the total lines.

**Immediate Actions Required**:
1. Create `gates/` subdirectory
2. Split into individual gate modules
3. Move tests to `tests/` directory
4. Target: each module ≤ 300 lines

**Estimated Refactoring Effort**: 4-6 hours
