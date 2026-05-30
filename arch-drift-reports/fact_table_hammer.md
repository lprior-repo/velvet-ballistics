# ARCH-DRIFT REPORT: fact_table.rs

**File**: `crates/vb_validate/src/fact_table.rs`  
**Lines**: 522 (MAXIMUM VIOLATION: 222 lines over limit)  
**Severity**: CRITICAL  
**Date**: 2026-05-29

---

## EXECUTIVE SUMMARY

This file violates the **<300 line rule** by 222 lines (74% over limit). The primary offender is a monolithic test module consuming 357 lines (68% of file). Secondary violations include primitive obsession throughout the domain logic and DDD boundary erosion.

---

## RESPONSIBILITY MAP

### Domain Logic (Production Code: 105 lines actual, 20% of file)

| Responsibility | Lines | Function(s) | Issue |
|----------------|-------|-------------|-------|
| Type requirement | 15-24 | `require_boolean` | Standalone free function |
| Value resolution | 27-41 | `resolve_value` | Mixed resolution logic |
| Composite resolution | 43-57 | `resolve_composite` | Nested iteration |
| Slot writing | 60-64 | `write_slot` | Raw index, no bounds typed |
| **Facts aggregate** | 67-101 | `Facts` struct + methods | **God struct**: holds 3 maps + resolution |
| Input facts building | 103-127 | `input_facts` | Duplicated entry pattern |
| Var facts building | 129-142 | `input_facts` | Duplicated entry pattern |
| Secret facts building | 144-157 | `secret_facts` | Duplicated entry pattern |
| Reference parsing | 159-164 | `reference_name` | Scattered string splitting |

### Tests (357 lines, 68% of file — **MUST EXTERNALIZE**)

| Test Group | Lines | Count |
|------------|-------|-------|
| Facts::build tests | 195-277 | 10 tests |
| require_boolean tests | 278-353 | 7 tests |
| resolve_value tests | 355-449 | 9 tests |
| write_slot tests | 451-471 | 2 tests |
| Edge cases | 473-521 | 3 tests |

---

## PRIMITIVE OBSESSION VIOLATIONS

### 1. Raw `String` Keys in HashMaps

```rust
// VIOLATION: HashMap<String, ValueFact>
inputs: HashMap<String, ValueFact>,
vars: HashMap<String, ValueFact>,
secrets: HashMap<String, ValueFact>,
```

**FIX**: Introduce `InputName`, `VarName`, `SecretName` wrapper types.

### 2. Raw `&str` Reference Parsing

```rust
// VIOLATION: Manual string manipulation in resolve_reference
let Some(body) = reference.strip_prefix('$') else { ... };
let Some((root, tail)) = body.split_once('.') else { ... };
let name = reference_name(tail);
```

**FIX**: Introduce `Reference` type with `root`, `tail` fields; `Reference::parse(str)` constructor.

### 3. Raw `usize` Slot Indices

```rust
// VIOLATION: Raw usize index
pub(crate) fn write_slot(slots: &mut [Option<ValueFact>], index: usize, fact: ValueFact)
```

**FIX**: Introduce `SlotIndex(u32)` newtype.

### 4. Raw `&[(String, ValueType)]` for Variables

```rust
// VIOLATION: Tuple instead of typed declaration
fn var_facts(vars: &[(String, ValueType)]) -> HashMap<String, ValueFact>
```

**FIX**: Introduce `VarDecl { name: VarName, value_type: ValueType }`.

### 5. Raw `&[String]` for Secrets

```rust
// VIOLATION: String slice instead of typed name
fn secret_facts(secrets: &[String]) -> HashMap<String, ValueFact>
```

**FIX**: Introduce `SecretName` wrapper.

### 6. Inline Error Construction with `to_owned()`

```rust
// VIOLATION: String allocation for error messages
Err(crate::ValidationError::TypeMismatch {
    expected: "boolean".to_owned(),
    found: actual.as_str().to_owned(),
})
```

**FIX**: `ValidationError` should carry `ValueType` directly, not converted to strings.

---

## DDD BOUNDARY VIOLATIONS

### 1. `Facts::resolve_reference` Does Too Much

The method performs:
- Prefix stripping (`$`)
- Root/tail splitting (`.`)
- Namespace dispatch (`input`, `var`, `secrets`)
- Map lookup
- Fallback default

**FIX**: Extract `Reference` value object with `parse()` method.

### 2. Duplicated HashMap Entry Pattern

Three nearly identical patterns:

```rust
match facts.entry(name.clone()) {
    std::collections::hash_map::Entry::Occupied(mut entry) => {
        entry.insert(ValueFact::clean(*vt));
    }
    std::collections::hash_map::Entry::Vacant(entry) => {
        entry.insert(ValueFact::clean(*vt));
    }
}
```

The `Occupied` arm **overwrites** (last-wins semantics), which is a **behavioral choice** that should be explicit, not buried in entry API misuse.

### 3. No Value Objects for Domain Identifiers

- `InputName`, `VarName`, `SecretName` are all raw `String`
- `Reference` is raw `&str`
- `SlotIndex` is raw `usize`

---

## STRUCTURAL DRIFT

### File Structure vs. Ideal

| Current | Ideal |
|---------|-------|
| 522 lines monolithic | Max 300 lines per file |
| Tests inline | `fact_table_test.rs` or `tests/` directory |
| Mixed domain + tests | Separate compilation units |

### Circular Dependency Risk

`Facts` depends on `WorkflowTypes` for construction but `WorkflowTypes` likely depends on types defined alongside `Facts`. This tight coupling suggests `Facts` should be in a lower-level module.

---

## PRESCRIBED REFACTORING

### Phase 1: Extract Tests (357 → 165 lines)

Move `#[cfg(test)]` module to `fact_table_test.rs` in parent `tests/` directory.

### Phase 2: Introduce Value Objects

```rust
// New types in type_sigs.rs
pub struct InputName(String);
pub struct VarName(String);
pub struct SecretName(String);
pub struct SlotIndex(u32);
pub struct Reference<'a> { root: &'a str, tail: &'a str }
```

### Phase 3: Refactor Facts

```rust
pub(crate) struct Facts {
    inputs: HashMap<InputName, ValueFact>,
    vars: HashMap<VarName, ValueFact>,
    secrets: HashMap<SecretName, ValueFact>,
}
```

### Phase 4: Extract Reference Parser

```rust
impl Reference<'_> {
    pub fn parse(s: &str) -> Option<Reference<'_>> { ... }
    pub fn root(&self) -> &str { ... }
    pub fn name(&self) -> &str { ... }
}
```

---

## VERDICT

**UNSHIPABLE** until:
1. Tests externalized to separate file (-357 lines → 165 lines)
2. Primitive obsessions resolved with domain types
3. DDD boundaries clarified with explicit value objects

---

## ESTIMATED REFACTORING COST

| Phase | Lines Removed | New Lines | Net |
|-------|---------------|------------|-----|
| Test extraction | -357 | +20 (imports) | -337 |
| Value objects | 0 | +80 | +80 |
| Reference extraction | 0 | +40 | +40 |
| **TOTAL** | **-357** | **+140** | **-217** |

**Result**: ~305 lines → comfortably under 300 line limit with room for growth.
