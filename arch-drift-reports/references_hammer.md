# ARCH-DRIFT REPORT: `vb_validate/src/references.rs`

**File**: `crates/vb_validate/src/references.rs`
**Line Count**: 845 lines (**VIOLATION: 281% over 300-line limit**)
**Status**: `REFACTOR REQUIRED`

---

## 1. LINE COUNT BREACH

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 845 | 300 | ❌ 281% OVER |
| Production code | ~237 | 300 | ✅ |
| Test code | ~608 | — | ❌ Tests should be in separate file |

**Verdict**: File MUST be split. The tests (lines 258–845) account for 587 lines — nearly 2× the entire budget — and belong in `references_tests.rs` behind a `#[cfg(test)]` module isolated in `tests/`.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS (Scott Wlaschin DDD)

### 2.1 `String` for All Name Types

Every name in the domain is stored as raw `String`:

```rust
pub struct RefTables {
    inputs: HashSet<String>,   // VIOLATION: should be HashSet<InputName>
    vars: HashSet<String>,    // VIOLATION: should be HashSet<VarName>
    secrets: HashSet<String>, // VIOLATION: should be HashSet<SecretName>
    step_ids: Vec<String>,    // VIOLATION: should be Vec<StepId>
    step_ids_set: HashSet<String>, // VIOLATION: should be HashSet<StepId>
}

pub struct WorkflowRefs {
    pub inputs: Vec<String>,     // VIOLATION
    pub vars: Vec<String>,       // VIOLATION
    pub secrets: Vec<String>,   // VIOLATION
    pub step_ids: Vec<String>,  // VIOLATION
    pub references: Vec<String>, // VIOLATION: should be Vec<Reference>
}
```

**Root Cause**: No NewType wrappers exist. `String` is used for 6 semantically distinct concepts:
- Input names
- Variable names
- Secret names
- Step identifiers
- Reference strings
- Bare reference words (`$now`, `$random`)

**Fix**: Create NewType wrappers:
```rust
pub struct InputName(String);
pub struct VarName(String);
pub struct SecretName(String);
pub struct StepId(String);
pub struct StepIndex(usize);
pub struct Reference(String);
pub struct RefRoot(String);  // "input", "var", "vars", "secrets", "step", "steps", "runtime"
```

### 2.2 `reference_name()` Returns Raw `&str`

```rust
fn reference_name(tail: &str) -> &str {
    match tail.split_once('.') {
        Some((name, _)) => name,
        None => tail,
    }
}
```

This function extracts the "name" part of a reference tail using string splitting — pure primitive obsession. It operates on unvalidated strings with no type safety.

**Fix**: Introduce `RefTail` value object with a constructor that validates and parses:
```rust
pub struct RefTail<'a>(&'a str);

impl<'a> RefTail<'a> {
    pub fn new(tail: &'a str) -> Option<Self> { ... }
    pub fn name(&self) -> &str { ... }
    pub fn field(&self) -> Option<&str> { ... }
}
```

### 2.3 Step Index is Raw `usize`

```rust
pub fn step_index(&self, step_id: &str) -> Option<usize> {
    self.step_ids.iter().position(|id| id == step_id)
}
```

`usize` is used for step indices throughout — no `StepIndex` NewType.

**Fix**: Wrap in `StepIndex(usize)` with bounded construction.

---

## 3. ANEMIC DOMAIN MODEL: `WorkflowRefs`

```rust
#[derive(Debug, Clone, Default)]
pub struct WorkflowRefs {
    pub inputs: Vec<String>,
    pub vars: Vec<String>,
    pub secrets: Vec<String>,
    pub step_ids: Vec<String>,
    pub references: Vec<String>,
}
```

`WorkflowRefs` is a pure data bag with:
- **No invariants enforced** — empty `inputs`, duplicate `step_ids`, malformed `references` all pass
- **No constructor validation** — `Default` is derivable; a completely empty `WorkflowRefs` is "valid"
- **No behavior** — all logic lives in free functions (`validate_references`, `validate_single_reference`)

**DDD Principle Violated**: "Types, not strings" — this struct should enforce that:
1. No duplicate names in any namespace
2. `references` contains only valid `$`-prefixed strings
3. `step_ids` has no duplicates

**Fix**: Replace with a proper aggregate:
```rust
pub struct WorkflowRefs {
    inputs: Vec<InputName>,
    vars: Vec<VarName>,
    secrets: Vec<SecretName>,
    step_ids: Vec<StepId>,
    references: Vec<Reference>,
}

impl WorkflowRefs {
    pub fn new(/* ... */) -> Result<Self, WorkflowRefsError> { ... }
    pub fn inputs(&self) -> &[InputName] { ... }
    // etc.
}
```

---

## 4. EXCESSIVE DUAL-NAMESPACE IN `RefTables`

```rust
pub struct RefTables {
    step_ids: Vec<String>,        // ordered list
    step_ids_set: HashSet<String>, // duplicate set for O(1) lookup
}
```

The same data is stored in two structures for two use cases. This is a code smell — it signals the `Vec` and `HashSet` roles aren't cleanly separated.

**Fix**: Keep ordered `Vec<StepId>` for index lookup; derive `HashSet<StepId>` on demand or use a single `Vec<StepId>` with binary search for existence checks (since step IDs are small in practice).

---

## 5. FUNCTION SIGNATURE PRIMITIVE OBSESSION

```rust
pub fn validate_single_reference(reference: &str, tables: &RefTables) -> ValidationResult<()>
pub fn validate_single_reference_with_context(
    reference: &str,
    tables: &RefTables,
    current_step_index: Option<usize>,
) -> ValidationResult<()>
```

Both take raw `&str` for `reference`. The caller must ensure proper `$`-prefixed strings are passed.

**Fix**: Replace `&str` with `&Reference` (a validated value object):
```rust
pub fn validate_single_reference(reference: &Reference, tables: &RefTables) -> ValidationResult<()>
```

---

## 6. RECOMMENDED FILE SPLIT

| File | Contents | Est. Lines |
|------|----------|------------|
| `types.rs` | NewTypes: `InputName`, `VarName`, `SecretName`, `StepId`, `StepIndex`, `Reference`, `RefTail` | ~120 |
| `ref_tables.rs` | `RefTables` struct + construction + lookup methods | ~80 |
| `validation.rs` | `validate_single_reference`, `validate_step_reference`, `validate_declared`, `parse_step_reference` | ~120 |
| `workflow_refs.rs` | `WorkflowRefs` domain model with constructor validation | ~60 |
| `references.rs` | Re-exports only; thin glue | ~20 |
| `references_tests.rs` | All tests moved here | ~580 |

**Total after split**: ~980 lines (mostly tests), production code ~400 lines across 5 files.

---

## 7. SUMMARY

| Violation | Severity | Fix |
|-----------|----------|-----|
| 845-line file (281% over limit) | 🔴 CRITICAL | Split into 6 files |
| `String` for 6 domain name types | 🔴 CRITICAL | NewType wrappers |
| `WorkflowRefs` is anemic | 🔴 CRITICAL | Aggregate with invariants |
| `reference_name()` uses raw strings | 🟡 HIGH | `RefTail` value object |
| Raw `usize` for step indices | 🟡 HIGH | `StepIndex` NewType |
| Dual Vec+HashSet for step_ids | 🟡 MEDIUM | Single source, derived lookups |
| `&str` in public API signatures | 🟡 MEDIUM | `Reference` value object |

---

## 8. MANDATORY ACTIONS

1. **Create `crates/vb_validate/src/types/`** directory with NewType definitions
2. **Move tests** to `crates/vb_validate/src/references_tests.rs`  
3. **Refactor `WorkflowRefs`** to enforce no duplicate names
4. **Replace all `String`** references to domain names with typed NewTypes
5. **Update `lib.rs`** to expose re-exports from split files

**ESTIMATED REFACTOR COMPLEXITY**: Medium — the validation logic itself is sound; the debt is in type safety, not algorithm correctness.
