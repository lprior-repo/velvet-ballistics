# Architectural Drift Report: `vb_validate/src/schema.rs`

**File:** `crates/vb_validate/src/schema.rs`  
**Total Lines:** 2195  
**Threshold:** 300 lines  
**Status:** 🚨 CRITICAL VIOLATION

---

## 1. Line Count Analysis

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 2195 | 300 | **+1895 over (731%)** |
| Production code | 451 | 300 | +151 over |
| Test code | 1744 | — | Inline, not separated |

**Breakdown:**
- Lines 1–451: Production code (validation logic + document model types)
- Lines 452–2195: Inline tests (`#[cfg(test)] mod tests`)

---

## 2. DDD Cohesion Check

**Filename:** `schema.rs`  
**Claimed domain concept:** Schema validation for workflow documents

**Verdict:** ⚠️ MODERATE SMELL — filename reflects a single concept, but file contains **two distinct DDD subdomains**:

| Concept | Lines | Role |
|---------|-------|------|
| Validation functions | 79–330 | Domain service layer |
| `WorkflowDoc`, `StepDoc`, `FieldValue` | 336–446 | Domain model (document structure) |

**Problem:** The document model types (`WorkflowDoc`, `StepDoc`, `FieldValue`) are in the same file as validation. These represent the **document structure** aggregate root — they belong in `src/model.rs` or `src/doc_model.rs`, not co-located with validation rules.

The validation functions are the **schema validation service**. The document model is a separate aggregate. They share a file because they both touch "schema," but they are two separate DDD bounded contexts.

---

## 3. All Violations

### V1: File Size — CRITICAL
- **Lines:** 2195
- **Limit:** 300
- **Severity:** MUST split

### V2: Inline Tests Not Separated
- **Lines:** 452–2195 (1744 lines)
- **Expected:** Tests should be in `tests/schema_validation_tests.rs` or `tests/schema_tests.rs`
- **Current:** `#[cfg(test)] mod tests` embedded in production file
- **Severity:** SHOULD move

### V3: Dual-Domain Cohesion Violation
- **Domain A:** Validation service (`validate_*` functions)
- **Domain B:** Document model (`WorkflowDoc`, `StepDoc`, `FieldValue`)
- **Problem:** These are two separate aggregates in DDD terms
- **Severity:** SHOULD split into `src/model.rs` + `src/validation.rs`

### V4: No Module Separation for Concerns
- **Constants** (lines 9–73): 6 separate `const` blocks defining field allowlists could be in `src/schema/constants.rs`
- **Validation logic** (lines 79–330): Could be `src/schema/validation.rs`
- **Document model** (lines 336–446): Could be `src/schema/model.rs`
- **Severity:** SHOULD refactor into sub-modules

### V5: Function Size (Borderline)
| Function | Lines | Assessment |
|----------|-------|------------|
| `validate_workflow_schema` | 10 | ✅ OK |
| `validate_trigger` | 31 | ⚠️ Consider split |
| `validate_ids` | 29 | ⚠️ Consider split |
| `validate_id` + `validate_single_id` | 12+12 | ✅ OK |
| `is_valid_id` | 18 | ✅ OK |
| `WorkflowDoc` impl | 58 | ⚠️ Large but acceptable for impl |
| `StepDoc` impl | 26 | ✅ OK |

No single function is egregious, but the file as a whole is far over budget.

---

## 4. Remediation Plan

### Priority 1 — URGENT (file split required)
```
schema.rs (2195 lines)
  ├── schema/
  │   ├── mod.rs          (~15 lines — re-exports)
  │   ├── constants.rs    (~75 lines — all const blocks)
  │   ├── model.rs        (~110 lines — WorkflowDoc, StepDoc, FieldValue)
  │   ├── validation.rs   (~250 lines — validate_* functions)
  │   └── id.rs           (~40 lines — is_valid_id, is_reserved_id, validate_id)
  └── tests/
      └── schema_validation.rs  (~1744 lines — moved from #[cfg(test)] mod tests)
```

### Priority 2 — DDD Cohesion
- Move `WorkflowDoc`, `StepDoc`, `FieldValue` to `schema/model.rs`
- Move validation functions to `schema/validation.rs`
- Keep `schema.rs` as a thin re-export module

### Priority 3 — Module Structure
```
vb_validate/src/
├── lib.rs
├── schema/
│   ├── mod.rs
│   ├── constants.rs   ← ALLOWED_TOP_LEVEL_FIELDS, ALLOWED_STEP_FIELDS, STEP_PRIMITIVES, RESERVED_IDS
│   ├── model.rs       ← WorkflowDoc, StepDoc, FieldValue
│   ├── id.rs          ← is_valid_id, is_reserved_id, validate_id, validate_single_id
│   ├── validation.rs  ← validate_workflow_schema, validate_trigger, validate_ids, etc.
│   └── triggers.rs    ← validate_empty_trigger, validate_named_string_trigger
└── tests/
    └── schema_validation.rs  ← All tests moved here
```

---

## 5. Summary

| Item | Status |
|------|--------|
| Lines count | 2195 (limit: 300) |
| DDD smell detected | YES — dual-domain in single file |
| Violations | 5 total (1 critical, 4 moderate) |
| Remediation priority | **HIGH** — file MUST be split |

**DDD Smell:** YES. The file conflates "schema validation" (a service) with "document model" (an entity/aggregate). These are two separate domain concepts that should live in separate modules under `schema/`.
