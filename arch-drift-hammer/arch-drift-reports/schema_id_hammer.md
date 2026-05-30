# Architectural Drift Report: `schema_id.rs`

**File**: `crates/vb_validate/src/schema_id.rs`
**Line Count**: 322 (VIOLATION: exceeds 300-line hard limit)
**Enforcer**: architectural-drift
**Date**: 2026-05-29

---

## Executive Summary

This file is **GUILTY** of multiple architectural drift violations including:
- **LINE COUNT**: 322 lines (over 300-line limit by 22 lines)
- **PRIMITIVE OBSESSION**: `&str` used where domain types should exist
- **DDD VIOLATIONS**: No value objects, anemic validation logic scattered across bare functions

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 322 | 300 | **FAIL** (+22) |

The file must be split. Recommended split:
- `schema_id/types.rs` — Value objects (`SchemaId`, `SchemaIdSet`, `ReservedId`)
- `schema_id/validation.rs` — Pure validation logic
- `schema_id/reserved.rs` — Reserved ID constants
- `schema_id/tests.rs` — Test module (already present but inline)

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 `&str` Used for Schema IDs (CRITICAL)

**Current**:
```rust
pub fn validate_single_id(id: &str, seen: &[&str]) -> ValidationResult<()>
pub fn is_valid_id(id: &str) -> bool
pub fn is_reserved_id(id: &str) -> bool
```

**Required** (DDD-prescribed):
```rust
// Newtype: makes illegal states unrepresentable
pub struct SchemaId(String);
pub struct SchemaIdSet(Set<SchemaId>);
pub struct ReservedId(String);

impl SchemaId {
    pub fn new(input: &str) -> Result<Self, SchemaIdError>;
    pub fn is_valid_format(&self) -> bool;
    pub fn is_reserved(&self) -> bool;
}
```

**Why this matters**:
- `&str` allows ANY string to be passed where only valid IDs are valid
- No compile-time enforcement that only valid IDs flow through the system
- `validate_single_id("BAD", &[])` compiles fine — should not

### 2.2 Magic Numbers Unnamed

**Current**:
```rust
if id.is_empty() || id.len() > 64  // 64 is magic
```

**Required**:
```rust
pub const SchemaId::MAX_LENGTH: usize = 64;
pub const SchemaId::MIN_LENGTH: usize = 1;
```

### 2.3 `seen: &[&str]` Should Be `SchemaIdSet`

**Current**:
```rust
pub fn validate_single_id(id: &str, seen: &[&str]) -> ValidationResult<()>
if seen.contains(&id) { ... }
```

**Required**:
```rust
pub struct SchemaIdSet { inner: Set<SchemaId> }
impl SchemaIdSet {
    pub fn contains(&self, id: &SchemaId) -> bool;
    pub fn insert(&mut self, id: SchemaId);
}
```

---

## 3. DDD VIOLATIONS (Scott Wlaschin Principles)

### 3.1 No Value Objects

| Primitive | Required Type | Violation |
|-----------|--------------|-----------|
| `&str` (ID) | `SchemaId` | Newtype wrapping String |
| `&[&str]` (seen) | `SchemaIdSet` | Set of SchemaId |
| `&str` (reserved) | `ReservedId` | Enum or newtype |

### 3.2 Anemic Validation Logic

The validation rules are scattered across bare functions instead of being encapsulated in domain types:

```rust
// WRONG: Logic split across standalone functions
pub fn is_valid_id(id: &str) -> bool { /* length + format */ }
pub fn is_reserved_id(id: &str) -> bool { /* lookup */ }
pub fn validate_single_id(id: &str, seen: &[&str]) -> ValidationResult<()>
```

**Correct DDD approach**:
```rust
impl SchemaId {
    pub fn try_from_input(raw: &str) -> Result<SchemaId, SchemaIdCreationError>;
    pub fn is_valid_format(&self) -> bool;
    pub fn is_reserved(&self) -> bool;
}

impl SchemaIdSet {
    pub fn validate_no_duplicates(&self, id: &SchemaId) -> Result<(), DuplicateIdError>;
}
```

### 3.3 ValidationError Should Be Domain Error Type

**Current**:
```rust
ValidationError::InvalidId { id: id.to_owned() }
ValidationError::ReservedId { id: id.to_owned() }
ValidationError::DuplicateId { id: id.to_owned() }
```

**DDD-prescribed**:
```rust
// Domain-specific error types in crate::schema_id::errors
pub enum SchemaIdError {
    Empty,
    TooLong { max: usize, actual: usize },
    InvalidFormat { char: char },
    Reserved,
    Duplicate,
}
```

---

## 4. RESERVED_IDS Constant Bloat

The `RESERVED_IDS` constant at line 42-74 is a flat array of 31 strings. This should be a proper set type with domain methods:

```rust
// Current: raw array
const RESERVED_IDS: &[&str] = &[...31 items...];

// DDD-prescribed:
pub struct ReservedIdSet(Set<CompactString>);
impl ReservedIdSet {
    pub fn is_reserved(&self, id: &SchemaId) -> bool;
}
```

---

## 5. TEST PRIMITIVE OBSESSION

Tests pass primitives directly, reinforcing the anti-pattern:

```rust
#[test]
fn is_valid_id_accepts_simple_lowercase() {
    assert!(is_valid_id("abc"));  // &str primitive
}
```

**DDD-prescribed**: Tests should use the value object:
```rust
#[test]
fn schema_id_accepts_simple_lowercase() {
    let id = SchemaId::new("abc").unwrap();
    assert!(id.is_valid_format());
}
```

---

## 6. SUMMARY OF VIOLATIONS

| # | Violation Type | Severity | Lines Affected |
|---|---------------|----------|----------------|
| 1 | Line count > 300 | **HARD FAIL** | 322 total |
| 2 | `&str` for SchemaId | CRITICAL | 7, 20, 38 |
| 3 | `&[&str]` for seen set | CRITICAL | 7, 14 |
| 4 | Magic number 64 | HIGH | 21 |
| 5 | No `SchemaId` value object | CRITICAL | All |
| 6 | No `SchemaIdSet` type | HIGH | 7 |
| 7 | No `ReservedId` type | HIGH | 38-39 |
| 8 | Flat constant array | MEDIUM | 42-74 |
| 9 | Validation scattered | HIGH | 7-18, 20-40 |
| 10 | Anemic domain errors | MEDIUM | 9, 12, 15 |

---

## 7. PRESCRIBED REFACTORING

### Target Structure
```
crates/vb_validate/src/schema_id/
├── mod.rs           (reexports)
├── schema_id.rs     (~50 lines)  — SchemaId value object
├── schema_id_set.rs (~40 lines)  — SchemaIdSet
├── reserved.rs      (~30 lines)  — ReservedIdSet
├── validation.rs    (~60 lines)  — Validation logic
└── errors.rs        (~40 lines)  — Domain errors
```

### Key Transformations

1. **Create `SchemaId` newtype**:
   - Wrap `String` (or `SmolStr` for interning)
   - `MAX_LENGTH = 64` as associated constant
   - `is_valid_format()` method
   - `is_reserved()` delegating to `ReservedIdSet`

2. **Create `SchemaIdSet`**:
   - Wrap `Set<SchemaId>` or `Vec<SchemaId>`
   - `insert()`, `contains()`, `validate_no_duplicates()`

3. **Create `ReservedIdSet`**:
   - Wrap `Set<ReservedId>` or sorted `Vec`
   - `is_reserved(id: &SchemaId) -> bool`

4. **Move validation into types**:
   - `SchemaId::try_from(raw: &str) -> Result<SchemaId, SchemaIdError>`
   - `SchemaIdSet::validate(id: &SchemaId) -> Result<(), DuplicateIdError>`

5. **Extract constants**:
   - `SchemaId::MAX_LENGTH = 64`
   - `SchemaId::MIN_LENGTH = 1`

---

## 8. VERDICT

**STATUS**: ARCHITECTURAL DRIFT **CONFIRMED**

This file is a textbook case of **primitive obsession** combined with **300-line violation**:
- Raw `&str` types instead of domain value objects
- Validation logic scattered across standalone functions
- No encapsulation of business rules
- Tests reinforcing the anti-pattern

**MANDATORY ACTIONS**:
1. Split into 4+ modules under `schema_id/` directory
2. Create `SchemaId` newtype immediately
3. Create `SchemaIdSet` type
4. Create domain error types
5. Move all tests to use new types

**ESTIMATED REFACTOR**: 2-3 beads

---

*Report generated by architectural-drift agent*
*Violations must be resolved before bead closure*
