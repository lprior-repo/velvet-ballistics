# Architectural Drift Report: `vb_storage/src/journal/incident.rs`

**File**: `crates/vb_storage/src/journal/incident.rs`  
**Analyzed**: 2026-05-29  
**Status**: VIOLATIONS FOUND

---

## 1. Line Count Check

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | **412** | 300 | ❌ OVER LIMIT |

**Breakdown**:
- Production code: ~173 lines (lines 1–172)
- Inline test module: ~239 lines (lines 173–412)

**Required Action**: Split tests into separate file. Move to `incident_tests.rs` in same directory or reference from `mod.rs`.

---

## 2. DDD Cohesion Analysis

### Domain Concept
**Bounded Context**: Workflow incident analysis and lifecycle state derivation  
**Ubiquitous Language**: SideEffect, IncidentAnalysis, FailureCode, LifecycleState, JournalEvent

### Cohesion Score: **MEDIUM**

The file is focused on a single domain concept (incident analysis from journal events), but suffers from:
1. Inline test module bloating (239 lines)
2. Type-system gap between domain types and implementation

### Entity/Value Object Inventory

| Name | Type | Location | Assessment |
|------|------|----------|------------|
| `SideEffect` | Struct (Value Object) | Line 12 | ⚠️ Uses `u16` primitives instead of `StepIdx`/`ActionId` |
| `SideEffectCertainty` | Enum | Line 20 | ✅ Good |
| `IncidentAnalysis` | Struct (Aggregate) | Line 27 | ⚠️ `failure_code: String` is primitive obsession |
| `analyze_incident_events` | Domain Function | Line 35 | ✅ Good |
| `build_repair_hints` | Domain Function | Line 84 | ✅ Good |
| `lifecycle_state_to_inspect_status` | Projection | Line 122 | ✅ Good |
| `derive_lifecycle_state_from_events` | Domain Function | Line 146 | ✅ Good |

---

## 3. Violations

### ❌ VIOLATION 1: Line Count Exceeded
- **Severity**: CRITICAL
- **Rule**: All `.rs` files must be under 300 lines
- **Current**: 412 lines
- **Fix**: Move `mod tests { ... }` (239 lines) to `incident_tests.rs`

### ⚠️ VIOLATION 2: Primitive Obsession — `failure_code: String`
- **Severity**: MEDIUM
- **Location**: Line 29
- **Problem**: `String` for failure code defeats `Parse, don't validate`. No type safety on failure codes.
- **Expected**: NewType like `FailureCode` enum or struct wrapping `&'static str`
- **Fix**: Define `enum FailureCode { RunFailed, RunCancelled }` or similar

### ⚠️ VIOLATION 3: Primitive Obsession — `SideEffect` fields
- **Severity**: LOW
- **Location**: Lines 13–14
- **Problem**: `step: u16`, `action: u16` instead of domain types `StepIdx`, `ActionId`
- **Note**: `vb_core::{ActionId, RunId, StepIdx}` are imported (line 8) but not used for these fields
- **Fix**: Change to `step: StepIdx, action: ActionId`

### ℹ️ VIOLATION 4: Test Placement
- **Severity**: INFO
- **Problem**: Inline `#[cfg(test)]` module is 239 lines. Per workspace structure, tests should be in `crates/workspace_tests/` or separate test files.
- **Fix**: Move to `incident_tests.rs`

---

## 4. DDD Smell Assessment

| Smell | Present | Location |
|-------|---------|----------|
| Primitive Obsession | ✅ Yes | `failure_code: String`, `step: u16`, `action: u16` |
| Data Drum | ⚠️ Partial | `IncidentAnalysis` bundles failure data, but is acceptable |
| State Machine | ✅ No | Lifecycle derivation is explicit match |
| Feature Envy | ✅ No | Functions operate on domain types |
| Invalid Message | ✅ No | All match arms are exhaustive |
| Shotgun Surgery | ✅ No | Single file, single concern |

**Overall Smell Level**: LOW-MEDIUM (mostly clean, primitive obsession issues)

---

## 5. Priority Recommendation

| Priority | Action | Effort |
|----------|--------|--------|
| **P1** | Split tests to `incident_tests.rs` | Low |
| **P2** | Replace `failure_code: String` with `FailureCode` enum | Low |
| **P3** | Use `StepIdx`/`ActionId` in `SideEffect` | Medium |

---

## 6. Summary

```
Lines:     412 / 300  ❌ OVER LIMIT
DDD:       Cohesive domain concept, primitive obsession on failure_code
Violations: 3 (1 critical, 2 medium/low)
Status:    STATUS: VIOLATIONS
```

**Recommended**: Fix P1 immediately (split tests), then address type-level issues.
