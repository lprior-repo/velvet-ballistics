# Architectural Drift Report: `vb_storage::recovery`

**Analyzed**: `crates/vb_storage/src/recovery/mod.rs`
**Date**: 2026-05-29
**Status**: DRIFT DETECTED

---

## 1. Line Count Analysis

| File | Lines | Limit | Status |
|------|-------|-------|--------|
| `mod.rs` | 64 | 300 | ✅ PASS |
| `replay/mod.rs` | 20 | 300 | ✅ PASS |
| `replay/attempt.rs` | 59 | 300 | ✅ PASS |
| `replay/core.rs` | 281 | 300 | ✅ PASS |
| `recover.rs` | 240 | 300 | ✅ PASS |
| **Production Code Violations** ||||
| `replay/summary.rs` | **1576** | 300 | ❌ **VIOLATION** |
| `types.rs` | **606** | 300 | ❌ **VIOLATION** |
| `hydrate_support.rs` | **599** | 300 | ❌ **VIOLATION** |
| `hydrate.rs` | **502** | 300 | ❌ **VIOLATION** |
| **Test Files** ||||
| `tests.rs` | 3432 | N/A | Excluded |
| `recovery_unit_tests.rs` | 1092 | N/A | Excluded |
| `vb_h6ix_tests.rs` | 930 | N/A | Excluded |

**Summary**: `mod.rs` itself is 64 lines (PASS). However, **4 production files exceed 300 lines**.

---

## 2. DDD Cohesion Analysis

### Facade Pattern (mod.rs) — PASS
The `mod.rs` correctly implements a **facade pattern** with clean re-exports:
- `types`: Recovery error types and state types
- `replay`: Core replay logic and event processing  
- `recover`: High-level recovery orchestration
- `hydrate`: Run frame hydration

### Cohesion Assessment

| Submodule | Responsibility | Cohesion |
|-----------|----------------|----------|
| `types.rs` | Value objects, errors, state enums | ✅ Cohesive |
| `replay/` | Event replay engine | ✅ Cohesive |
| `recover.rs` | High-level recovery orchestration | ✅ Cohesive |
| `hydrate.rs` | Frame hydration | ⚠️ Borderline |
| `hydrate_support.rs` | Hydration support (likely utilities) | ⚠️ Needs review |

**NewType Usage**: Cannot assess from `mod.rs` alone; requires submodule inspection.

**Workflow Modeling**: The module documents explicit state transitions (snapshot-plus-tail recovery, full journal recovery) which aligns with DDD workflow patterns.

---

## 3. Violations

### CRITICAL (Priority 1)
1. **`replay/summary.rs` — 1576 lines (425% over limit)**
   - Massive file; likely contains multiple responsibilities
   - Must be split into `replay/summary/*.rs` submodules

### HIGH (Priority 2)
2. **`types.rs` — 606 lines (102% over limit)**
   - Possible primitive obsession violations
   - Should be analyzed for Value Object extraction

3. **`hydrate_support.rs` — 599 lines (100% over limit)**
   - Hydration support utilities may be mixed
   - Check for `Parse, don't validate` adherence

### MEDIUM (Priority 3)
4. **`hydrate.rs` — 502 lines (67% over limit)**
   - Frame hydration logic may need refactoring
   - Verify single responsibility principle

---

## 4. DDD Smells

| Smell | Evidence | Severity |
|-------|----------|----------|
| **File Bloat** | 4 files >300 lines | High |
| **Potential Primitive Obsession** | Cannot assess from facade | Unknown |
| **Leaky Abstraction** | `hydrate_support.rs` name suggests utilities leaked | Medium |

---

## 5. Summary

| Metric | Value |
|--------|-------|
| `mod.rs` lines | 64 ✅ |
| Production files violating limit | 4 ❌ |
| Total production code lines (recovery/) | ~5,400 |
| DDD Cohesion (module level) | ✅ Clean facade |
| DDD Cohesion (file level) | ⚠️ 4 violations |

---

## 6. Recommendations

1. **Split `replay/summary.rs`** into:
   - `replay/summary/event_summary.rs`
   - `replay/summary/recovery_summary.rs`
   - `replay/summary/mod.rs`

2. **Audit `types.rs`** for primitive obsession (String IDs, i32 indices)

3. **Review `hydrate_support.rs`** for misplaced responsibilities

4. **Re-run architectural-drift gate** after splits

---

**Priority**: HIGH — File bloat violations require immediate attention
**DDD Smell Level**: MODERATE — Cohesive at module level, but file sizes indicate potential single-responsibility violations
