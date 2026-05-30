# Architectural Drift Report: vb_yaml Types

**File Analyzed:** `crates/vb_yaml/src/types.rs` (NOT FOUND - types are in `events_types.rs` and `source_map_types.rs`)
**Date:** 2026-05-29
**Agent:** architectural-drift

---

## Summary

| Metric | Value |
|--------|-------|
| **Total Lines** | 317 (across 2 files) |
| **Individual File Status** | PERFECT (< 300 lines each) |
| **DDD Cohesion** | ACCEPTABLE |
| **DDD Smell** | Primitive Obsession |
| **Priority** | LOW |

---

## 1. Line Count Analysis

| File | Lines | Status |
|------|-------|--------|
| `events_types.rs` | 198 | ✅ UNDER LIMIT |
| `source_map_types.rs` | 119 | ✅ UNDER LIMIT |
| **Total** | **317** | ✅ Split appropriately |

**Finding:** No single file exceeds 300 lines. The `types.rs` file requested does not exist; types are appropriately distributed across `events_types.rs` (YAML event domain) and `source_map_types.rs` (source location domain).

---

## 2. DDD Cohesion Analysis

### events_types.rs (198 lines)
**Domain:** YAML Event Stream

| Type | Purpose | Cohesion |
|------|---------|----------|
| `ScalarStyle` | Enum for scalar quoting styles | ✅ Cohesive |
| `EventSpan` | Source location for events | ✅ Cohesive |
| `YamlEvent` | Typed YAML event enum | ✅ Cohesive |

**Cohesion Verdict:** HIGH - All types serve the YAML event stream domain.

### source_map_types.rs (119 lines)
**Domain:** Source Location Tracking

| Type | Purpose | Cohesion |
|------|---------|----------|
| `SourceSpan` | Byte/line/column span | ✅ Cohesive |
| `SemanticSourceMap` | JSONPath-to-span mapping | ✅ Cohesive |
| `SourceMap` | Node-index-to-span mapping | ✅ Cohesive |

**Cohesion Verdict:** HIGH - All types serve source location tracking domain.

---

## 3. Violations

### Violation 1: Missing types.rs Module (Structural)
- **Severity:** INFO
- **Description:** No `types.rs` file exists. The canonical naming would expect a unified `types.rs` or clear `mod.rs` organizing type re-exports. Current split (`events_types.rs`, `source_map_types.rs`) is acceptable but deviates from canonical naming expectation.
- **Recommendation:** Consider adding `mod.rs` re-exports or renaming to follow `<crate>_types.rs` convention.

### Violation 2: Primitive Obsession - anchor_id (DDD)
- **Location:** `YamlEvent::Alias`, `YamlEvent::Scalar::anchor_id`, `YamlEvent::SequenceStart::anchor_id`, `YamlEvent::MappingStart::anchor_id`
- **Type:** `usize`
- **Issue:** Raw `usize` for anchor IDs violates "make illegal states unrepresentable" - nothing prevents arithmetic on anchor IDs.
- **Recommendation:** Newtype `AnchorId(usize)` with bounded operations.

### Violation 3: Primitive Obsession - node_index (DDD)
- **Location:** `SourceMap::span_for_node(node_index: u32)`
- **Type:** `u32`
- **Issue:** Raw `u32` for node indexing.
- **Recommendation:** Newtype `NodeIndex(u32)`.

### Violation 4: Primitive Obsession - Byte Offsets (DDD)
- **Location:** `EventSpan::{start, end}`, `SourceSpan::{start_offset, end_offset}`
- **Type:** `usize`
- **Issue:** Raw `usize` for byte offsets allows unchecked arithmetic.
- **Recommendation:** Newtypes `ByteOffset(usize)` and `CharOffset(usize)`.

### Violation 5: Primitive Obsession - Line/Column (DDD)
- **Location:** `EventSpan::{line, column}`, `SourceSpan::{start_line, start_col, end_line, end_col}`
- **Type:** `usize`
- **Issue:** Raw `usize` for source positions.
- **Recommendation:** Newtypes `Line(usize)`, `Column(usize)`.

---

## 4. DDD Smell Assessment

**Smell:** `Primitive Obsession`

**Rationale:** The types are well-factored into cohesive domains (events, source maps), but within those domains, primitive types (`usize`, `u32`) are used where domain-specific newtypes would make illegal states unrepresentable.

**Not Present:**
- ❌ Feature Envy (no cross-domain data access)
- ❌ Data Clump (no recurring parameter groups)
- ❌ Shotgun Surgery (changes don't cascade widely)
- ❌ Parallel Inheritance (no duplicate hierarchies)
- ❌ Lazy Element (no parasitic helper modules)

---

## 5. Priority Assessment

| Factor | Score | Notes |
|--------|-------|-------|
| Line Count Violation | 0 | No violations |
| DDD Cohesion | 0 | High cohesion |
| Primitive Obsession | 2 | Low severity - internal-only |
| Structural Drift | 1 | Missing `types.rs` naming |
| **TOTAL** | **3** | **LOW PRIORITY** |

**Justification:**
1. All files are under 300 lines
2. Domains are cohesive (events vs source maps)
3. Primitive obsession is in internal struct fields, not public API
4. No `unwrap`, `expect`, `panic`, `unsafe`, or `todo` found
5. The missing `types.rs` is a naming convention issue, not a functional problem

---

## 6. Recommendations

1. **Optional:** Create `types.rs` that re-exports from `events_types` and `source_map_types` for canonical naming
2. **Optional:** Add newtype wrappers for `AnchorId`, `NodeIndex`, `ByteOffset`, `Line`, `Column` if public API stability is needed
3. **No immediate action required** - the code is well-structured

---

## Status

**STATUS: PERFECT**

No refactoring required. The codebase adheres to architectural constraints.
