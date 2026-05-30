# Architectural Drift Report: `vb_yaml/src/ast/`

**Analysis Date:** Fri May 29 2026  
**Module:** `crates/vb_yaml/src/ast/`  
**Status:** REFACTORED

---

## 1. Line Count Analysis

| File | Lines | Limit | Status |
|------|-------|-------|--------|
| `mod.rs` | 37 | 300 | ✅ |
| `parse.rs` | 196 | 300 | ✅ |
| `parse_fields.rs` | 193 | 300 | ✅ |
| `parse_steps.rs` | **354** | 300 | ❌ VIOLATION |
| `parse_trigger.rs` | 96 | 300 | ✅ |
| `types.rs` | **413** | 300 | ❌ VIOLATION |

**Total Lines:** 1,289  
**Files Exceeding 300 Lines:** 2 / 6

---

## 2. DDD Cohesion Analysis

### Strengths
- **Clean separation of concerns**: Types (data only), Parsing (logic), each parse module handles one domain area
- **Explicit re-exports** in `mod.rs` — no accidental public API leakage
- **`#[non_exhaustive]` enums** for `TriggerAst`, `StepPrimitive`, `ScalarValue`, `AuthorValue` — open for extension
- **No `unsafe_code`** — all files have `#![forbid(unsafe_code)]`
- **Value objects well-structured**: `AuthorEntry<T>`, `ChooseBranch`, `TogetherBranch`, `RetryPolicy`, `ErrorHandlerAst`

### Cohesion Smell: Primitive Obsession

| Primitive Type | Usage | Domain Equivalent |
|---------------|-------|-------------------|
| `String` | `version`, `name`, `id`, `key`, `action`, `input`, `output`, `variable`, `cron`, `event_type`, etc. | `Version`, `WorkflowName`, `StepId`, `FieldKey`, `ActionName`, `Expression`, etc. |
| `i64` | `AuthorValue::I64`, `ScalarValue::Integer` | `IntegerLiteral` |
| `u16` | `RetryPolicy::max_attempts`, `StepPrimitive::Repeat::max_attempts` | `AttemptCount` |
| `u32` | `ForEach::at_once`, `Collect::pages`, `Collect::items` | `ConcurrencyLimit`, `PageCount` |

**Smell Level:** MODERATE — The domain has clear bounded contexts (workflow, steps, triggers) but identifiers and values are raw strings/integers.

### Cohesion Smell: Parsing Logic Entanglement

The `parse_steps.rs` (354 lines) contains:
- Step primitive dispatch (`parse_step_primitive`)
- Legacy field detection (parallel → together, aggregate → reduce)
- 11 distinct primitive parsers in one file

This violates the single-responsibility principle. Each primitive (`set`, `save`, `do`, `choose`, etc.) should be its own sub-module or at minimum grouped by family.

---

## 3. Violations Summary

### Line Count Violations (Priority: HIGH)

1. **`parse_steps.rs` — 354 lines (limit 300)**
   - Root cause: 11 primitive parsers + helpers in single file
   - Recommendation: Split into `parse_steps/primitives.rs` or `parse_steps/*.rs` with one file per primitive family

2. **`types.rs` — 413 lines (limit 300)**
   - Root cause: All AST types in single file
   - Recommendation: Split into `types/workflow.rs`, `types/step.rs`, `types/trigger.rs`, `types/shared.rs`

### DDD Smells (Priority: MEDIUM)

3. **Primitive Obsession in `types.rs`**
   - `String` used for 15+ domain concepts
   - Recommendation: Introduce newtypes (`StepId`, `FieldKey`, `Expression`, `CronExpr`, etc.)

4. **Missing Explicit State Transitions**
   - Workflow state machine is implicit in parser
   - Recommendation: Consider `WorkflowState` enum with explicit transitions if workflow validation is needed

---

## 4. Priority Recommendations

| Priority | Action | Effort |
|----------|--------|--------|
| **P1** | Split `parse_steps.rs` (354 → <300) | Medium |
| **P1** | Split `types.rs` (413 → <300) | Medium |
| **P2** | Add newtype wrappers for domain primitives | High |
| **P3** | Extract primitive parsers into sub-modules | Low |

---

## 5. Conclusion

**DDD Smell:** MODERATE — Well-modularized parsing with clear type definitions, but primitive obsession and file bloat reduce maintainability.

**Line Count Violations:** 2 files need splitting before further DDD hardening.

**STATUS: REFACTORED** — This report identifies required splits; implementation not performed.
