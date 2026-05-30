# Architectural Drift Report: `vb_yaml/src/ast/parse.rs`

**File**: `crates/vb_yaml/src/ast/parse.rs`  
**Date**: 2026-05-29  
**Agent**: architectural-drift

---

## 1. Line Count

| Metric | Value | Status |
|--------|-------|--------|
| Total lines | 196 | ✅ PASS (< 300) |

---

## 2. DDD Cohesion Analysis

### Cohesion Score: MODERATE

This module serves as both a **parsing entry point** and a **YAML manipulation utility library**. The cohesion is split:

| Responsibility | Functions | Cohesion |
|----------------|-----------|----------|
| Entry points | `parse_workflow_ast`, `parse_workflow_from_yaml` | High (workflow-specific) |
| YAML helpers | `lookup`, `mapping`, `sequence`, `require_str`, etc. | Low (generic) |

### Generic Helpers (Low Cohesion with Module Purpose)
These helpers are reusable YAML utilities that don't belong exclusively to this domain:
- `lookup`, `mapping`, `sequence`
- `require_str`, `require_str_in`, `require_scalar_in`
- `opt_str`, `opt_u32`, `require_u16`
- `reject_unknown_fields`

These are **primitive-obsessed** utilities that should ideally be in a `yaml_helpers` or `yaml_ext` module, not co-located with domain parsing logic.

---

## 3. Violations

### ❌ Primitive Obsession

| Field | Type Used | DDD Smell |
|-------|-----------|-----------|
| `version` | `String` | Should be `Version` (NewType) |
| `name` | `String` | Should be `WorkflowName` (NewType) |
| `field` | `&'static str` | Acceptable for error context |
| `u16`/`u32` | Raw integers | Should be typed (e.g., `Port`, `Timeout`) |

**Impact**: No compile-time enforcement that version strings are valid, names conform to expected format, etc.

### ❌ Low Cohesion Helpers

The helper functions (lines 24-140) are generic YAML operations that don't express domain concepts. They mix:
- Generic: `lookup`, `mapping`, `sequence`  
- Domain-adjacent: `require_str`, `require_u16`

**Recommendation**: Extract to `crates/vb_yaml/src/ast/yaml_helpers.rs` or similar.

### ❌ Hardcoded Field Names

In `reject_unknown_fields`:
```rust
for (key, _) in mapping(node, "mapping")? {  // "mapping" hardcoded
```
This silently assumes the node has a "mapping" field when it's used for error reporting.

### ⚠️ Missing Domain State Machine

The `parse_workflow_from_yaml` function manually assembles `WorkflowSourceParts` but there's no explicit state machine modeling workflow states/transitions.

---

## 4. DDD Smell Summary

| Smell | Severity | Location |
|-------|----------|----------|
| Primitive Obsession | MEDIUM | Throughout - raw `String`, `u16`, `u32` |
| Low Cohesion | MEDIUM | Lines 24-140 (generic helpers) |
| No NewTypes | LOW | `version`, `name` fields |
| Hidden Error Context | LOW | `reject_unknown_fields` field name assumption |

---

## 5. Priority Assessment

| Priority | Issue | Rationale |
|----------|-------|-----------|
| **P2** | Extract generic YAML helpers | Improves cohesion, enables reuse |
| **P3** | Add NewTypes for `Version`, `WorkflowName` | Prevents invalid states at type level |
| **P4** | Add domain state machine modeling | Future-proofs workflow transitions |

---

## 6. Recommendations

1. **Short-term**: Extract lines 24-140 to `yaml_ext.rs` module
2. **Medium-term**: Introduce NewType wrappers for `version`, `name`
3. **Long-term**: Model workflow parse state as explicit state machine

---

**STATUS**: MINOR_REFACTOR_NEEDED  
**Files Affected**: `crates/vb_yaml/src/ast/mod.rs` (if helpers extracted)
