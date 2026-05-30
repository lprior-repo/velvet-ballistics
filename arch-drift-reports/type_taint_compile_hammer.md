# Architectural Drift Report: `type_taint.rs`

**File:** `crates/vb_compile/src/type_taint.rs`
**Total Lines:** 514 (TARGET: <300)
**Overflow:** +214 lines (71% over limit)

---

## Executive Summary

This module performs type-and-taint inference validation for workflow ASTs. It is a **600-pound primitive-obsessed god module** that fuses domain logic, state management, fact tracking, expression evaluation, and string-parsing into a single undigestible file. The DDD cohesion is near-zero; the primitive obsession is near-maximum.

---

## 1. LINE COUNT VIOLATION

| Metric | Value |
|--------|-------|
| Current lines | 514 |
| Target | 300 |
| Overflow | 214 (71.3%) |
| Single largest function | `validate_steps` (66 lines) |
| Module block count | 1 `#[cfg(test)]` block |

**Root Cause:** This file should be 3-4 separate modules, not one monolithic validator.

---

## 2. RESPONSIBILITY MAPPING

### Current Responsibilities (conflated)
1. **Value type lattice** — `ValueType` enum + `ValueFact` struct
2. **Fact state machine** — `Facts` struct with slot/input/var/secret tracking
3. **Expression fact inference** — `expression_fact`, `parsed_expression_fact`, `unary_fact`, `binary_fact`, `helper_fact`
4. **Stringly-typed reference parsing** — `reference_fact` (pure string manipulation)
5. **Schema fact construction** — `input_schema_fact`, `schema_mapping_fact`, `schema_type`
6. **Workflow step validation** — `validate_steps` dispatch
7. **Condition/result validation** — `validate_condition`, `validate_public_result`

### Correct DDD Boundaries
```
type_taint/
├── domain/           # ValueType lattice, Taint domain concepts
│   ├── value_type.rs # ValueType enum + type-safe constructors
│   └── taint.rs      # Taint merge logic (if not imported)
├── facts/            # Fact state management
│   ├── mod.rs        # Facts struct
│   └── slot.rs       # SlotIndex wrapper type
├── expression/       # Expression fact inference
│   ├── mod.rs        # expression_fact dispatch
│   ├── binary.rs     # binary_fact
│   ├── unary.rs      # unary_fact
│   └── helper.rs     # helper_fact, helper_type
├── reference/        # String reference parsing
│   └── mod.rs        # reference_fact → ReferencePath struct
├── schema/           # AST-to-fact conversion
│   └── mod.rs        # input_schema_fact, schema_type
└── validation/       # Workflow-level validation
    ├── mod.rs        # validate_workflow_ast, validate_steps
    └── public_result.rs # validate_public_result
```

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3.1 `ValueType` — Stringly-Typed Type Lattice

**Location:** Lines 13-36, 138-147

```rust
// VIOLATION: Raw string comparison for type names
fn schema_type(name: &str) -> ValueType {
    match name {
        "text" => ValueType::Text,
        "number" => ValueType::Number,
        // ...
    }
}
```

**Problem:** Schema type names are matched via raw string comparison. This should be a type-safe constructor on `ValueType`.

**Fix:** `ValueType::from_schema_name(name)` with a `TryFrom<&str>` implementation.

---

### 3.2 Field Names as Raw `'static str`

**Locations:** Throughout `validate_steps`, `validate_condition`, `expression_fact`, etc.

```rust
// VIOLATION: Stringly-typed field identifiers
facts.read_slot(input.as_usize(), "run.input")
facts.read_slot(input.as_usize(), "for_each.input")
validate_condition(condition, facts)?;  // hardcoded "choose.condition"
validate_public_result(result, facts)?;  // hardcoded "finish.result"
```

**Problem:** Every field reference is a raw string literal. Typos are not caught at compile time. No discoverability.

**Fix:** Typed field identifier enum:

```rust
#[derive(Copy, Clone, Debug)]
enum StepField {
    RunInput,
    ForEachInput,
    ChooseCondition,
    ReduceInput,
    // ...
}

impl StepField {
    const fn as_str(self) -> &'static str { ... }
}
```

---

### 3.3 Slot Index as Raw `usize`

**Locations:** `Facts::slots: Vec<Option<ValueFact>>`, `write_slot`, `read_slot`, `as_usize()` calls throughout.

```rust
// VIOLATION: Unchecked usize slot index
fn write_slot(&mut self, index: usize, fact: ValueFact) {
    if let Some(slot) = self.slots.get_mut(index) {  // silent failure on OOB
        *slot = Some(fact);
    }
}
```

**Problem:** Out-of-bounds slot access silently fails (no error returned). Callers use `as_usize()` on `NonZeroU64` wrappers scattered across the call site.

**Fix:** `SlotIndex(u64)` wrapper with checked `new()` and `get()` methods.

---

### 3.4 `reference_fact` — Pure String Manipulation in Domain Logic

**Location:** Lines 487-511

```rust
// VIOLATION: String parsing in domain logic
fn reference_fact(reference: &str, facts: Option<&Facts<'_>>) -> ValueFact {
    let Some(body) = reference.strip_prefix('$') else {
        return ValueFact::clean(ValueType::Text);
    };
    let Some((root, tail)) = body.split_once('.') else {
        return ValueFact::clean(ValueType::Any);
    };
    // ... more string splitting
}
```

**Problem:** This is a pure string parser wearing a fact function's clothing. It should be a separate `ReferencePath` struct with `parse`, `root()`, `tail()`, `resolve()` methods.

**Fix:**
```rust
struct ReferencePath<'a> {
    root: ReferenceRoot,
    segments: Vec<&'a str>,
}

enum ReferenceRoot { Input, Var, Secrets }
```

---

### 3.5 `HashMap<&'a str, ValueFact>` — String Keys

**Locations:** Lines 72-75, 102-115, 149-162, 164-183

```rust
// VIOLATION: String-keyed maps for domain concepts
struct Facts<'a> {
    inputs: HashMap<&'a str, ValueFact>,
    vars: HashMap<&'a str, ValueFact>,
    secrets: HashMap<&'a str, ValueFact>,
}
```

**Problem:** `&str` keys are not type-safe. Name collisions, typos, and invalid identifiers are not caught at the type level.

**Fix:** Strongly-typed identifier wrappers:
```rust
struct InputIdent<'a>(pub &'a str);
struct VarIdent<'a>(pub &'a str);
// ...
struct Facts<'a> {
    inputs: HashMap<InputIdent<'a>, ValueFact>,
    vars: HashMap<VarIdent<'a>, ValueFact>,
    secrets: HashMap<SecretIdent<'a>, ValueFact>,
}
```

---

### 3.6 Unused Generic Parameter in `secret_facts`

**Location:** Lines 164-183

```rust
// VIOLATION: Generic T never used
fn secret_facts<T>(entries: &[AstMapEntry<T>]) -> HashMap<&str, ValueFact> {
```

**Problem:** `T` is a dead generic. This function takes `&[AstMapEntry<T>]` but only reads `entry.name`, ignoring the `value` entirely. This is dead code or a design error.

**Fix:** Remove the generic or rename to `_T` if intentional.

---

## 4. DDD COHESION FAILURES

### 4.1 Low Cohesion — One File Does Everything

The module has 0 cohesion. It contains:
- A domain lattice (`ValueType`)
- A state model (`Facts`)
- Pure functions (`schema_type`, `matches_type`, `first_mismatch`)
- String parsers (`reference_fact`)
- Validation dispatch (`validate_steps`)
- Expression visitors (`expression_fact`, `parsed_expression_fact`)
- Helper dispatch (`helper_fact`, `helper_taint`, `helper_type`)

**Scott Wlaschin Rule:** A module should have one responsibility. This has 8.

### 4.2 Anemic Domain Model

`ValueFact` is a pure data bag with no methods beyond constructors and one merge operation. The behavioral logic is scattered in standalone functions.

**Fix:** Move behavior onto `ValueFact`:
```rust
impl ValueFact {
    fn merge(self, other: Self) -> Self { ... }
    fn with_type(self, vt: ValueType) -> Self { ... }
    fn is_compatible_with(self, expected: ValueType) -> bool { ... }
}
```

---

## 5. SPECIFIC REFACTORING TARGETS

| Line | Issue | Target |
|------|-------|--------|
| 13-36 | `ValueType` needs `TryFrom<&str>` | `domain/value_type.rs` |
| 72-75 | String-keyed maps | `facts/state.rs` with typed idents |
| 88-99 | Slot index as raw `usize` | `facts/slot.rs` |
| 102-147 | Schema fact construction | `schema/mod.rs` |
| 185-251 | `validate_steps` too large | `validation/steps.rs` + step-specific validators |
| 290-301 | `expression_fact` dispatch | `expression/mod.rs` |
| 303-417 | Expression visitors | `expression/` submodule |
| 487-511 | `reference_fact` string parsing | `reference/mod.rs` + `ReferencePath` |

---

## 6. EXTRACTED MODULE COUNT (Proposed)

| Module | Est. Lines |
|--------|------------|
| `domain/value_type.rs` | 40 |
| `facts/mod.rs` | 50 |
| `facts/slot.rs` | 25 |
| `schema/mod.rs` | 45 |
| `expression/mod.rs` | 30 |
| `expression/binary.rs` | 25 |
| `expression/unary.rs` | 25 |
| `expression/helper.rs` | 40 |
| `reference/mod.rs` | 40 |
| `validation/mod.rs` | 60 |
| **Total** | **~380** |

Still over 300 due to inherent complexity. Further splitting required:
- Move step-specific validators to `validation/steps/` (one file per step kind)
- Abstract `Facts` state into a trait for testability

---

## 7. SEVERITY ASSESSMENT

| Violation | Severity | Effort to Fix |
|-----------|----------|---------------|
| Line count overflow | **CRITICAL** | High (full module decomposition) |
| `ValueType` primitive obsession | **HIGH** | Medium |
| Slot index unchecked `usize` | **HIGH** | Medium |
| `reference_fact` string parsing | **MEDIUM** | Medium |
| Field names as raw `str` | **MEDIUM** | Medium |
| `HashMap<&str, _>` string keys | **MEDIUM** | Low |
| `secret_facts<T>` dead generic | **LOW** | Trivial |

---

## 8. RECOMMENDED REFACTORING ORDER

1. **Extract `ValueType` + `TryFrom<&str>`** — smallest blast radius, establishes domain type
2. **Create `SlotIndex(u64)` wrapper** — fixes silent OOB, reduces `as_usize()` calls
3. **Extract `ReferencePath` struct** — isolates string parsing from fact logic
4. **Extract `schema/` module** — `input_schema_fact`, `schema_mapping_fact`, `schema_type`
5. **Extract `expression/` module** — all expression visitor functions
6. **Extract `facts/` module** — `Facts` struct with typed identifiers
7. **Extract `validation/` module** — `validate_steps` + step validators
8. **Inline `secret_facts<T>`** — remove dead generic

---

## Conclusion

This file is a **structural violation of DDD cohesion principles**. It treats `&str`, `usize`, and `HashMap<&str, _>` as first-class domain concepts when they should be hidden behind typed wrappers. The 514-line monolith must be decomposed into at least 6 modules before it can be considered architecturally compliant.

**Next Action:** Create `crates/vb_compile/src/type_taint/` directory and begin module extraction starting with `domain/value_type.rs`.
