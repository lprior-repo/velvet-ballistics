# ARCHITECTURAL DRIFT REPORT: part_05.rs

**File**: `crates/vb_compile/src/mod_compile_lowering/part_05.rs`
**Line Count**: 410 lines (VIOLATION: >300 limit)
**Status**: CRITICAL DRIFT DETECTED

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 410 | 300 | **VIOLATION (+110 lines)** |
| Functions | 10 | N/A | Acceptable |
| Match Arms (digest_step_primitive) | 15 | N/A | UNACCEPTABLE |

---

## 2. RESPONSIBILITY MAPPING

This file performs **5 distinct responsibilities** crammed into one file:

| # | Responsibility | Lines | Domain Concept |
|---|----------------|-------|----------------|
| 1 | **Primitive Parsing** | 15-114 | YAML → typed values |
| 2 | **Workflow Digestion** | 129-335 | Content-addressable identity |
| 3 | **Branch Count Validation** | 163-193 | Together cardinality bounds |
| 4 | **Step Lowering to IR** | 342-367 | Compilation output |
| 5 | **Primitive Lowering Helpers** | 370-404 | Set/Do node construction |

**Assessment**: Violates Single Responsibility Principle. Each concern deserves its own module.

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3.1 Raw `i64` in `parse_i64_field` (line 20)

```rust
pub(super) fn parse_i64_field(
    value: &str,
    step: usize,
    field: &'static str,
) -> Result<i64, CompileErrors> {
    value.parse::<i64>().map_err(...)
}
```

**Problem**: Returns raw `i64` instead of a domain value object.

**DDD Fix**: Create a typed value object like `WorkflowVersion` or `TimeoutMs`:
```rust
pub(super) fn parse_version(value: &str) -> Result<WorkflowVersion, CompileErrors>;
```

### 3.2 Unvalidated Index Conversions

**Locations**: Lines 48-50, 87-90, 176, 249

```rust
let raw = u16::try_from(value)
    .map_err(|_| CompileErrors(vec![CompileError::SlotIndexOutOfRange { value }]))?;
Ok(SlotIdx::new(raw))
```

**Problem**: Each call site repeats the same `u16::try_from` + error construction pattern. This is copy-paste validation, not abstraction.

**DDD Fix**: Create a `SlotIdx::from_i64_checked(value)` constructor or a `TryFrom<i64> for SlotIdx` impl in the type's home module.

### 3.3 `StepIdxSlotExt` Trait (lines 64-72)

```rust
pub(super) trait StepIdxSlotExt {
    fn to_slot(self) -> SlotIdx;
}
```

**Problem**: A one-method extension trait to convert between two index types is a code smell. This indicates the types live in the wrong modules or lack a shared abstraction.

**DDD Fix**: Either:
- Move this to where `StepIdx` and `SlotIdx` are defined
- Create a shared `IndexLike` trait in `vb_core`
- Simply call `SlotIdx::new(self.get())` at call sites (less abstraction, more clarity)

### 3.4 Raw `usize` for Branch Counts

**Location**: Line 176
```rust
if branches.len() > usize::from(u16::MAX) {
```

**Problem**: `usize::from(u16::MAX)` is a magic conversion repeated across the codebase. Should be a named constant.

---

## 4. SCOTT WLASCHIN DDD VIOLATIONS

### 4.1 `digest_step_primitive` is a 132-line Goblin Function

**Location**: Lines 194-326

This function is a **15-arm match statement** doing:
- Blake3 hashing of every StepPrimitive variant
- Manual byte serialization of each field
- Recursive calls for nested structures
- Embedded validation (u16 bounds check at line 249)

**Problems**:
1. **Too many responsibilities**: Hashing + recursion + validation
2. **Copy-paste hashing pattern**: Each variant manually calls `hasher.update()`
3. **No abstraction for "hash this primitive part"**
4. **Cyclomatic complexity**: 15 arms = impossible to verify exhaustiveness visually

**Refactor Direction**:
```rust
// Each primitive should hash itself via a trait
trait HashablePrimitive {
    fn hash_into(&self, hasher: &mut blake3::Hasher) -> Result<(), CompileErrors>;
}

// digest_step_primitive becomes 5 lines:
pub fn digest_step_primitive(hasher: &mut blake3::Hasher, p: &StepPrimitive) -> Result<(), CompileErrors> {
    p.hash_into(hasher)
}
```

### 4.2 `canonical_primitive_name` is Redundant with `Debug`/`Display`

**Location**: Lines 98-114

```rust
pub(crate) fn canonical_primitive_name(primitive: &vb_yaml::ast::StepPrimitive) -> &'static str {
    match primitive {
        vb_yaml::ast::StepPrimitive::Set { .. } => "set",
        ...
    }
}
```

**Problem**: This exists SOLELY to keep the digest in sync with human-readable names. This is a **brittle synchronization requirement** that should be automated.

**DDD Fix**: Derive `&'static str` from the variant name itself:
```rust
fn variant_name<T: 'static>() -> &'static str { ... }
```

Or encode the name in the variant: `StepPrimitive::Set { name: &'static str, ... }`

### 4.3 Validation and Hashing are Inappropriately Coupled

**Locations**: 
- `validate_branch_counts` (lines 163-170)
- `canonical_digest` (lines 129-155) calls `validate_branch_counts` first

**Problem**: The digest function calls validation BEFORE hashing. This means:
- You cannot compute a digest for an invalid workflow (even for error reporting)
- Validation and digest are coupled at call site

**Scott Wlaschin says**: "Make illegal states unrepresentable." If branch count validation is a hard constraint, it should be enforced at type level (a `Together<N>` type where `N: ArrayBounds<16>`), not at runtime.

### 4.4 No Value Objects for Domain Concepts

| Raw Type | Should Be |
|----------|-----------|
| `&str` (step id) | `StepIdentifier` (newtype) |
| `i64` (timeout) | `Timeout` (typed with units) |
| `&str` (variable name) | `VariableName` |
| `&str` (prompt) | `PromptText` |

---

## 5. SPECIFIC CODE QUALITY ISSUES

### 5.1 Dead Code Path (line 93)
```rust
_ => Err(CompileErrors(vec![
    CompileError::UnsupportedConstantValue { step: 0 },
])),
```
`step: 0` is a magic number. Should be the actual step context.

### 5.2 Clippy Suppression Missing (line 341)
```rust
#[allow(clippy::too_many_arguments)]
```
7 arguments to `lower_steps_to_ir` is a smell. Should be a config struct:
```rust
pub struct LoweringConfig {
    pub slot_count: u16,
    pub symbols_count: u32,
    pub name: Box<str>,
    pub digest: WorkflowDigest,
}
```

### 5.3 `canonical_finish_slot` Return Type Inconsistency

Lines 79-85 return `SlotIdx` via `outputs.get(name).copied()` (Option)
Lines 86-90 return `SlotIdx` directly via `Ok(SlotIdx::new(raw))`

The mismatch requires different error handling strategies at call sites.

---

## 6. TEST PLACEMENT VIOLATION

**Location**: Lines 406-410
```rust
#[cfg(test)]
#[path = "../tests/digest_unit_tests.rs"]
mod tests;
```

**Problem**: The comment says "keep this file under the 300-line limit" but this is achieved by OUTSOURCING tests to another file. This is **gaming the metric**, not fixing the problem.

**True fix**: The file is still 410 lines of production code. The line limit applies to production code.

---

## 7. RECOMMENDATIONS (Priority Order)

| Priority | Action | Impact |
|----------|--------|--------|
| **P0** | Split `digest_step_primitive` into per-primitive `hash_into()` trait impls | Reduces this file by ~100 lines |
| **P0** | Move `parse_i64_field`, `slot_from_text` to a `yaml/parsing` submodule | Reduces this file by ~50 lines |
| **P1** | Create `SlotIdx::try_from_i64(value)` in vb_core | Eliminates repeated validation |
| **P1** | Replace `#[allow(clippy::too_many_arguments)]` with config struct | Reduces coupling |
| **P2** | Create `TimeoutMs`, `WorkflowVersion` value objects | Eliminates primitive obsession |
| **P2** | Remove `StepIdxSlotExt`, inline `SlotIdx::new(self.get())` | Less indirection |
| **P3** | Extract branch validation into a separate module | Single responsibility |

---

## 8. VERDICT

**ARCHITECTURAL DRIFT: CRITICAL**

This file demonstrates:
1. ✅ Primitive obsession (raw i64, u16, &str泛滥)
2. ✅ God function (`digest_step_primitive`: 132 lines, 15 arms)
3. ✅ Mixed responsibilities (parsing + validation + hashing + lowering)
4. ✅ Gaming the line count (tests outsourced, not actual reduction)
5. ❌ Under 300 lines (410 lines)

**Required Action**: Decompose into at minimum 3 modules:
- `compile/lowering/digest.rs` (hashing)
- `compile/lowering/parse.rs` (YAML parsing)
- `compile/lowering/ir.rs` (lowering to IR)
