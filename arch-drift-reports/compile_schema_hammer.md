# Architectural Drift Report: `vb_compile/src/schema.rs`

**File**: `crates/vb_compile/src/schema.rs`  
**Total Lines**: 735  
**Status**: 🔴 REFACTOR REQUIRED

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total lines | 735 | 300 | 🔴 VIOLATION (+435) |
| Production code | ~515 | 300 | 🔴 VIOLATION (+215) |
| Test code | ~220 | N/A | Should be split |

**Required Split**:
1. `schema/validation.rs` — Core validation logic (lines 1-385)
2. `schema/types.rs` — `SchemaKind`, `SchemaScope`, bound types (lines 26-72, 366-486)
3. `schema/kind.rs` — SchemaKind parsing (lines 48-71)
4. `schema/bounds.rs` — Bound validation (lines 366-486)
5. `schema/tests.rs` — Tests (lines 516-735)

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Field Name Primitives (`&str`)

**Lines 156-173**: Raw string literals for field names
```rust
const FIELDS: &[&str] = &[
    "is", "of", "fields", "extra", "optional", "nullable",
    "default", "min", "max", "min_length", "max_length", "pattern", "secret",
];
```
**Violation**: `&str` for domain concept "schema field name"  
**Fix**: NewType `SchemaFieldName(&str)` with `From<&str>` only for valid values

**Lines 97-110**: Raw string literals for schema shorthand
```rust
fn is_schema_shorthand(value: &str) -> bool {
    matches!(value, "text"|"number"|"boolean"|"object"|"any"|"list<any>"...)
}
```
**Violation**: `&str` instead of `SchemaShorthand` enum  
**Fix**: Parse into `enum SchemaShorthand { Text, Number, Boolean, Object, List(list_item) }`

### 2.2 Numeric Primitives (`i64`)

**Lines 373-401**: Untyped bounds
```rust
fn validate_min_max_bounds(mapping: &saphyr::Mapping<'_>, kind: SchemaKind) -> Vec<CompileError> {
    let min = match optional_integer_schema_field(mapping, "min") { ... };
    let max = match optional_integer_schema_field(mapping, "max") { ... };
```
**Violation**: `i64` for min/max values instead of `SchemaBound`, `ListBound`, `TextLengthBound`  
**Fix**: NewTypes with validation in constructor

**Lines 475-486**: Raw `i64` returned from `optional_integer_schema_field`
```rust
fn optional_integer_schema_field(...) -> Result<Option<i64>, CompileError>
```
**Violation**: No domain type for schema integers  
**Fix**: `SchemaInteger(Option<i64>)` or `NonNegativeInteger`

### 2.3 YAML Value Primitives

**Lines 505-510**: `mapping_get` returns raw `Yaml<'_>`
```rust
fn mapping_get<'a>(mapping: &'a saphyr::Mapping<'a>, field: &str) -> Option<&'a Yaml<'a>>
```
**Violation**: `Yaml` is a generic tree node, not a domain type  
**Fix**: Domain-specific optional accessors returning typed `Option<SchemaValue<'_>>`

---

## 3. SCOTT WLASCHIN DDD VIOLATIONS

### 3.1 Validation vs. Parse Don't Validate

**Lines 86-95, 97-110**: `validate_schema_shorthand` + `is_schema_shorthand`
```rust
fn validate_schema_shorthand(value: &str) -> Vec<CompileError> {
    if is_schema_shorthand(value) { Vec::new() } else { ... }
}
fn is_schema_shorthand(value: &str) -> bool {
    matches!(value, "text"|"number"|...)
}
```
**Violation**: "Validate, don't parse" is backwards. Should be "Parse, don't validate"  
**Fix**: Try to parse into `SchemaShorthand` enum; if `None`, return error

### 3.2 Missing Value Objects

The file has no value objects for:
- `SchemaFieldName` — Validated string for field names
- `SchemaBound` — i64 with min/max relationship enforced
- `SchemaTextLength` — Non-negative integer for text length
- `SchemaListLength` — Non-negative integer for list length
- `SchemaShorthand` — Parsed shorthand notation
- `SchemaValue` — Typed YAML value wrapper

### 3.3 Anemic Domain Model

**Lines 26-46**: Enums exist but are passive
```rust
enum SchemaKind { Text, Number, Boolean, Object, List, Any }
enum SchemaScope { Input, ObjectField }
```
**Violation**: Enums have methods but no behavior; validation is all procedural  
**Fix**: Move validation behavior into domain types via typed constructors

### 3.4 Workflow State Machine Missing

The validation functions at lines 112-129 show validation order but don't model state:
```rust
fn validate_schema_mapping(mapping: &saphyr::Mapping<'_>, scope: SchemaScope) -> Vec<CompileError> {
    errors.append(&mut reject_unknown_schema_fields(mapping, scope));
    errors.append(&mut reject_schema_pattern(mapping));
    errors.append(&mut validate_schema_from(mapping, scope));
    // ... kind detection, then children validation
```
**Violation**: State transitions are implicit in function call order, not explicit  
**Fix**: Model as `SchemaValidationState` with explicit transitions

---

## 4. MIXED CONCERNS

### 4.1 Test Code Inline (Lines 516-735)

The test module is **220 lines** mixed into production code.

**Violations**:
- `VB_YD5X_*` byte string constants (564-652) are 88 lines of YAML in Rust source
- `vb_yd5x_validate_via_compile` (655-659) calls `vb_validate::shared::validate` — cross-crate integration test logic in unit test module
- `first_compile_code` (661-669) is a test helper that doesn't belong in schema validation

**Fix**: Move to `crates/vb_compile/tests/schema_validation_tests.rs`

### 4.2 Cross-Crate Test Dependency

**Line 658**:
```rust
vb_validate::shared::validate(&parts)
```
**Violation**: `vb_compile` depends on `vb_validate` for integration testing; this is a circular/shared dependency pattern that shouldn't be in unit tests

---

## 5. FUNCTION COMPLEXITY ISSUES

### 5.1 Long Functions (Not >30 lines but worth noting)

| Function | Lines | Status |
|----------|-------|--------|
| `validate_schema_mapping` | 18 | ⚠️ Acceptable but aggregates |
| `validate_min_max_bounds` | 18 | ⚠️ Acceptable but aggregates |
| `validate_schema_bounds` | 5 | ✅ |

### 5.2 Repetitive Pattern

**Lines 373-401, 414-438, 462-473**: Same bound-order validation pattern repeated
```rust
let min = match optional_integer_schema_field(mapping, "min") { ... };
let max = match optional_integer_schema_field(mapping, "max") { ... };
if min.is_none() && max.is_none() { return Vec::new(); }
// validate kind
// validate values
// validate order
```
**Fix**: Generic `validate_optional_bounds<T>(min: Option<T>, max: Option<T>, ...)`

---

## 6. SPECIFIC REFACTORING OBLIGATIONS

| ID | Location | Issue | Fix |
|----|----------|-------|-----|
| DR-001 | Lines 156-173 | `&str` field names | `struct SchemaFieldName(&str)` |
| DR-002 | Lines 97-110 | String shorthand | `enum SchemaShorthand` parser |
| DR-003 | Lines 475-486 | Raw `i64` | `struct SchemaInteger(i64)` |
| DR-004 | Lines 366-486 | Scattered bounds | `mod bounds { struct SchemaBound, struct TextLength, struct ListLength }` |
| DR-005 | Lines 516-735 | Inline tests | Move to `tests/schema_validation.rs` |
| DR-006 | Lines 505-510 | `mapping_get` raw `Yaml` | `trait SchemaMapping { fn get_field(&self, name: SchemaFieldName) -> Option<SchemaValue<'_>> }` |
| DR-007 | Lines 86-95 | Validate not parse | Replace with `SchemaShorthand::parse(value: &str) -> Result<SchemaShorthand, CompileError>` |

---

## 7. SUMMARY

| Category | Count | Severity |
|----------|-------|----------|
| Line count violations | 1 | 🔴 CRITICAL |
| Primitive obsession | 4 | 🔴 CRITICAL |
| DDD violations | 4 | 🔴 CRITICAL |
| Mixed concerns | 2 | 🟡 MEDIUM |
| Function issues | 2 | ⚠️ LOW |

**Total Violations**: 13  
**Estimated Refactor Cost**: 2-3 beads

---

## 8. RECOMMENDED SPLIT

```
crates/vb_compile/src/schema/
├── mod.rs           # Re-exports
├── validation.rs    # validate_input_schema, validate_schema_mapping, validate_schema_* (lines 5-130, 131-321, 323-365)
├── kind.rs          # SchemaKind, SchemaScope (lines 26-72)
├── bounds.rs        # Bound validation, optional_integer_schema_field (lines 366-510)
├── shorthand.rs    # Schema shorthand parsing (lines 86-110)
├── yaml_utils.rs    # mapping_get, yaml_bool, invalid_schema (lines 488-514)
└── tests.rs         # Lines 516-735 (moved from schema.rs)
```

**Status**: 🔴 **REFACTOR REQUIRED** — File exceeds 300 line threshold by 435 lines and contains multiple primitive obsession violations requiring NewType encapsulation.
