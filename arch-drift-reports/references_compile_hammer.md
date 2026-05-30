# ARCH-DRIFT REPORT: `vb_compile/src/references.rs`

**File**: `crates/vb_compile/src/references.rs`
**Line Count**: 360 lines (**VIOLATION: 20% over 300-line limit**)
**Status**: `REFACTOR REQUIRED`

---

## 1. LINE COUNT BREACH

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 360 | 300 | ❌ 20% OVER |
| Production code | ~337 | 300 | ❌ 12% OVER |
| Test code | 0 (empty mod) | — | ⚠️ Dead module declaration |

**Verdict**: File MUST be refactored. At minimum, split into a reference-validation module (~180L) and a reference-types module (~120L). The `collect_references_from_*` family of near-identical traversal functions (lines 63-206) is ~140 lines of duplication that can be consolidated.

---

## 2. RESPONSIBILITY MAP

```
references.rs
├── validate_workflow_ast()                    [Lines 14-29] — Entry point
├── build_ref_tables()                        [Lines 31-37] — Table construction
├── Name extractors (3 identical patterns)    [Lines 39-61]
│   ├── entry_names_owned<T>()               — Vec<AstMapEntry<T>> → Vec<String>
│   ├── secret_names_owned()                  — Vec<AstMapEntry<Box<str>>> → Vec<String>
│   └── step_names_owned()                    — Vec<StepAst> → Vec<String>
├── AST traversal / reference collection       [Lines 63-206] (~140 lines)
│   ├── collect_references_from_value_entries()
│   ├── collect_references_from_expression_entries()
│   ├── collect_references_from_values()
│   ├── collect_references_from_steps()
│   ├── collect_references_from_step_kind()
│   ├── collect_references_from_expression()
│   ├── collect_references_from_parsed_expression()
│   └── collect_references_from_value()
├── Reference validation                       [Lines 208-312]
│   ├── validate_compile_reference()          — Main dispatch
│   ├── validate_slot_reference()             — Slot-specific validation
│   ├── numeric_accessor_path()              — Helper
│   └── check_accessor_path()                 — Accessor path checking
└── Error mapping                              [Lines 316-357]
    └── map_validation_error()                — vb_validate → CompileError

```

**Bounded Context**: `vb_compile` — compile-time reference validation for workflow ASTs. Delegates to `vb_validate::references` for shared reference semantics but handles compile-specific slot references locally.

---

## 3. PRIMITIVE OBSESSION VIOLATIONS (Scott Wlaschin DDD)

### 3.1 `&str` for References Throughout

```rust
// Line 213 — raw &str
fn validate_compile_reference(
    reference: &str,   // VIOLATION: should be &Reference
    tables: &RefTables,
    step_index: Option<usize>,
) -> Result<(), CompileError>

// Lines 240, 287-288 — raw &str for parsed components
fn validate_slot_reference(reference: &str, root: &str, tail: &str)
fn check_accessor_path(reference: &str, root: &str, tail: &str, tables: &RefTables)
```

**Problem**: `&str` is used for 4 semantically distinct concepts:
- Full reference string (`$vars.data.field`)
- Reference root (`vars`, `input`, `secrets`, `slot`, `slots`)
- Reference tail (`data.field` after root)
- Accessor path (`field` after declared name)

**Fix**: Introduce value objects:
```rust
pub struct Reference<'a>(&'a str);           // Validated "$"-prefixed reference
pub struct RefRoot<'a>(&'a str);             // "vars", "input", "slot", etc.
pub struct RefTail<'a>(&'a str);             // "data.field"
pub struct AccessorPath<'a>(&'a str);        // "field" or "0.1.2"
```

### 3.2 `Option<usize>` for Step Index

```rust
// Lines 67, 78, 89, 99, 110, 140, 162, 190 — ALL use raw Option<usize>
fn collect_references_from_value_entries(
    entries: &[AstMapEntry<AstValue>],
    tables: &RefTables,
    errors: &mut Vec<CompileError>,
    step_index: Option<usize>,   // VIOLATION
)
```

**Problem**: `usize` is unconstrained. A step index of `usize::MAX` is technically valid but nonsensical in this domain.

**Fix**: NewType wrapper with bounded construction:
```rust
pub struct StepIndex(usize);

impl StepIndex {
    pub fn new(idx: usize, max_steps: usize) -> Option<Self> {
        if idx < max_steps { Some(Self(idx)) } else { None }
    }
    pub fn get(&self) -> usize { self.0 }
}
```

### 3.3 `u16` and `u32` for Numeric Slots/Accessors

```rust
// Line 245
if slot.parse::<u16>().is_err() { ... }   // VIOLATION: should use SlotIdx from vb_core

// Line 273
if segment.parse::<u32>().is_err() { ... } // VIOLATION: should be AccessorIndex
```

**Problem**: The crate already imports `SlotIdx` and `StepIdx` from `vb_core` (seen in `ast/types.rs`), but `references.rs` uses raw numeric types instead.

**Fix**: Use imported types:
```rust
use vb_core::{SlotIdx, StepIdx};  // Already imported in ast/types.rs
// Or create local aliases if compile-crate separation requires it:
pub type LocalSlotIdx = u16;
```

### 3.4 String Formatting for Path Construction

```rust
// Lines 256, 304 — manual string concatenation
let accessor_root = format!("{root}.{slot}");  // VIOLATION
let accessor_root = format!("{root}.{name}");  // VIOLATION
```

**Problem**: Path segments are manually concatenated instead of using a proper `AccessorRoot` type with a `to_accessors()` method.

**Fix**:
```rust
pub struct AccessorRoot<'a> {
    root: &'a str,
    name: &'a str,
}

impl<'a> AccessorRoot<'a> {
    pub fn new(root: &'a str, name: &'a str) -> Self { Self { root, name } }
    pub fn as_str(&self) -> String { format!("{}.{}", self.root, self.name) }
}
```

### 3.5 `Box<str>` in AST Types But `String` Everywhere Else

The AST correctly uses `Box<str>` for memory efficiency:
```rust
// ast/types.rs correctly uses Box<str>
pub struct AstMapEntry<T> {
    pub name: Box<str>,   // ✅ Correct
    pub value: T,
}
```

But `references.rs` allocates intermediate `String` values:
```rust
// Lines 40-44 — unnecessary String allocation
fn entry_names_owned<T>(entries: &[AstMapEntry<T>]) -> Vec<String> {
    let mut names = Vec::with_capacity(entries.len());
    for entry in entries {
        names.push(entry.name.as_ref().to_owned());  // Allocates String
    }
    names
}
```

**Fix**: Use iterator-based extraction without allocation, or work with `&[Box<str>]` slices:
```rust
fn entry_names_slice<'a, T>(entries: &'a [AstMapEntry<T>]) -> Vec<&'a str> {
    entries.iter().map(|e| e.name.as_ref()).collect()
}
```

---

## 4. DUPLICATED TRAVERSAL LOGIC (140+ lines)

The `collect_references_from_*` family has 8 functions with nearly identical signatures:

| Function | Lines | Lines of Code |
|----------|-------|---------------|
| `collect_references_from_value_entries` | 63-72 | 9 |
| `collect_references_from_expression_entries` | 74-83 | 9 |
| `collect_references_from_values` | 85-94 | 9 |
| `collect_references_from_steps` | 96-104 | 8 |
| `collect_references_from_step_kind` | 106-134 | 28 |
| `collect_references_from_expression` | 136-156 | 20 |
| `collect_references_from_parsed_expression` | 158-184 | 26 |
| `collect_references_from_value` | 186-206 | 20 |

**Total: ~140 lines of copy-paste traversal logic**

**Root Cause**: Each function does the same pattern-matching recursion but calls a different "leaf" validator. This is the Visitor pattern without the Visitor.

**Fix**: Consolidate into a single generic traversal with a trait:
```rust
trait ReferenceVisitor {
    fn visit_reference(&mut self, reference: &str);
}

struct CollectRefsVisitor<'a> {
    tables: &'a RefTables,
    errors: &'a mut Vec<CompileError>,
    step_index: Option<usize>,
}

impl ReferenceVisitor for CollectRefsVisitor<'_> {
    fn visit_reference(&mut self, reference: &str) {
        if let Err(e) = validate_compile_reference(reference, self.tables, self.step_index) {
            self.errors.push(e);
        }
    }
}
```

---

## 5. FUNCTION SIGNATURE ANTI-PATTERNS

### 5.1 Boolean Parameter with `Option<usize>`

```rust
// Line 220 — implicit boolean via split result
let Some((root, tail)) = body.split_once('.') else {
    return Ok(());  // Implicit "no dot = valid bare reference"
};
```

The bare-reference case is handled by `split_once` returning `None` — clever but obfuscated.

### 5.2 Inconsistent Error Handling

```rust
// Lines 217-223 — early return for bare references
let Some(body) = reference.strip_prefix('$') else {
    return Ok(());  // Bare references pass silently
};
```

But elsewhere:
```rust
// Line 303-309 — returns error for declared names with accessor paths
if is_declared {
    return Some(CompileError::UnsupportedAccessorReference { ... });
}
```

**Inconsistency**: `$vars` (bare) is valid, `$vars.data` (with accessor) is invalid. This business rule is buried in code flow, not expressed as a named constant or type.

---

## 6. RECOMMENDED FILE SPLIT

| File | Contents | Est. Lines |
|------|----------|------------|
| `references/types.rs` | NewTypes: `RefRoot`, `RefTail`, `AccessorPath`, `StepIndex` | ~80 |
| `references/validation.rs` | `validate_compile_reference`, `validate_slot_reference`, `check_accessor_path`, `map_validation_error` | ~100 |
| `references/traversal.rs` | Consolidated visitor + `collect_references_from_*` | ~80 |
| `references.rs` | Re-exports + `validate_workflow_ast`, `build_ref_tables` | ~50 |

**Total after split**: ~310 lines (over budget) — further extraction of name extractors or error variants needed.

---

## 7. SUMMARY

| Violation | Severity | Fix |
|-----------|----------|-----|
| 360-line file (20% over limit) | 🔴 CRITICAL | Split into 4 files |
| `&str` for 4 reference concepts | 🔴 CRITICAL | `Reference`, `RefRoot`, `RefTail`, `AccessorPath` NewTypes |
| `Option<usize>` for step_index | 🟡 HIGH | `StepIndex` NewType |
| `u16`/`u32` for slot/accessor indices | 🟡 HIGH | Use `SlotIdx` from `vb_core` |
| 140 lines of duplicated traversal | 🟡 HIGH | Generic visitor trait |
| Manual string formatting for paths | 🟡 MEDIUM | `AccessorRoot` value object |
| Empty `#[cfg(test)] mod tests` | 🟡 MEDIUM | Remove dead module declaration |
| Unnecessary `String` allocations | 🟡 MEDIUM | Work with `&[Box<str>]` slices |

---

## 8. MANDATORY ACTIONS

1. **Create `crates/vb_compile/src/references/`** directory
2. **Extract `types.rs`** — define `RefRoot`, `RefTail`, `AccessorPath`, `StepIndex` NewTypes
3. **Extract `validation.rs`** — move validation functions
4. **Extract `traversal.rs`** — consolidate 8 `collect_references_from_*` into visitor trait
5. **Replace all `&str` references** in function signatures with typed value objects
6. **Remove empty `#[cfg(test)] mod tests;`** declaration
7. **Update `lib.rs`** — change `mod references` to `mod references;` with re-exports

**ESTIMATED REFACTOR COMPLEXITY**: Medium — the validation logic is sound; the debt is entirely in type safety and code organization.
