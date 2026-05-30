# Architectural Drift Report: `vb_compile/src/schema.rs`

**File**: `crates/vb_compile/src/schema.rs`  
**Total Lines**: 735  
**Limit**: 300  
**Status**: VIOLATION

---

## Line Count Violation

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 735 | 300 | **FAIL** (+435 over limit) |
| Code Lines | ~515 | - | - |
| Test Lines | ~220 | - | - |

---

## DDD Cohesion Analysis

### Cohesion Score: LOW

This file exhibits **Low Cohesion** — it attempts to serve multiple DDD concerns within a single module:

| Concern | Present | Location |
|---------|---------|----------|
| Schema validation orchestration | ✅ | `validate_input_schemas` |
| Schema type modeling (Value Objects) | ✅ | `SchemaScope`, `SchemaKind` |
| Field validation (per-field rules) | ✅ | `validate_schema_*` functions |
| Helper/utilities | ✅ | `mapping_get`, `yaml_bool` |
| Integration tests | ✅ | `mod tests` (220 lines) |

**Verdict**: This is a "God Module" — one file doing too many jobs.

---

## Violations

### 1. File Size Violation (CRITICAL)
- **Lines**: 735 (limit 300)
- **Overflow**: 435 lines over limit
- **Severity**: MUST fix

### 2. Low DDD Cohesion (HIGH)
- Multiple domain concepts jammed into one file
- Should be split into:
  - `schema/types.rs` — `SchemaScope`, `SchemaKind` value objects
  - `schema/validation.rs` — validation logic
  - `schema/helpers.rs` — `mapping_get`, `yaml_bool`, etc.
  - `schema/tests.rs` or `schema/integration_tests.rs` — tests

### 3. Primitive Obsession (MEDIUM)
- `&str` used directly for field names (should be `SchemaFieldName` newtype)
- `i64` used for bounds (should be `NonNegativeI64` or similar)
- `bool` used for flags without type aliasing

### 4. Parallel Validation Chains (CODE SMELL)
- Functions like `validate_schema_bounds` call `validate_min_max_bounds` AND `validate_text_length_bounds` separately, appending errors
- This pattern suggests the validation is doing "double duty" — could be unified

---

## DDD Smell Summary

| Smell | Severity | Description |
|-------|----------|-------------|
| God File | CRITICAL | 735 lines in single module |
| Low Cohesion | HIGH | Multiple DDD concepts per file |
| Primitive Obsession | MEDIUM | Raw `&str`, `i64`, `bool` |
| Parallel Validation | LOW | Repeated error-collection patterns |

---

## Recommendations

1. **Split immediately** into:
   - `schema.rs` — re-exports only
   - `schema/types.rs` — `SchemaScope`, `SchemaKind`
   - `schema/validation.rs` — all `validate_*` functions
   - `schema/helpers.rs` — `mapping_get`, `yaml_bool`, etc.
   - `schema/validation_tests.rs` — tests (move from inline `mod tests`)

2. **Newtypes for primitives**:
   ```rust
   pub struct SchemaFieldName(Box<str>);
   pub struct NonNegativeI64(i64);
   ```

3. **Consolidate error collection** — use a builder pattern or `Vec::extend` more efficiently

---

## Priority: **HIGH**

File is 245% over the line limit and demonstrates poor DDD cohesion. Refactoring is mandatory before further feature work.
