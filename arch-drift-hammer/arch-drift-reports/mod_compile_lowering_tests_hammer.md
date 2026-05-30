# ARCHITECTURAL DRIFT REPORT
**File**: `crates/vb_compile/src/mod_compile_lowering/tests.rs`
**Lines**: 1410 (LIMIT: 300)
**Status**: REFACTOR REQUIRED

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Violation |
|--------|-------|-------|-----------|
| Total lines | 1410 | 300 | **1170 lines over (470%)** |

This file is **not a test module** — it is **five distinct test suites** crammed into one file.

---

## 2. RESPONSIBILITY MAPPING

The file contains **6 separate test suites** with distinct domain responsibilities:

| Lines | Suite | Domain Concept Under Test |
|-------|-------|--------------------------|
| 1–250 | Collect Digest Coverage | `Collect` field hashing via `compute_compiled_digest` |
| 258–515 | Direct Digest Primitive | `digest_step_primitive` on `StepPrimitive::Collect` |
| 520–755 | Choose Width/Overflow | `choose_width`, `body_width`, overflow guards |
| 759–903 | Body Offset Overflow | `add_body_offset`, `emit_choose_branch_body` |
| 910–1067 | Slot Text Parsing | `slot_from_text` boundary/error cases |
| 1070–1410 | Choose Lowering | `lower_choose`, `lower_canonical_choose`, `SlotCompiler` |

**Each suite tests a different function/area.** They share only the import block.

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3.1 `&str` Used Where NewTypes Exist

```rust
// VIOLATION: Lines 29-69, 259-273
fn collect_yaml_with_field(
    variable: &str,       // Should be `VariableName`
    source: &str,         // Should be `SourceName` or similar
    pages: Option<u32>,   // Should be `PageCount`
    items: Option<u32>,   // Should be `ItemCount`
    body_var: Option<&str>,
) -> Vec<u8>             // Should be `YamlSource` or `CompiledSource`
```

The domain has `vb_core::ids` types (e.g., `StepIdx`, `SlotIdx`) but the test helpers use raw `&str` and `u32`.

### 3.2 Raw Integer Constants Without Names

```rust
// VIOLATION: Lines 580, 719, 1079 — hardcoded magic numbers
(0..64).map(|i| ...)           // Magic number 64 should be MAX_BRANCHES
(0..65).map(|i| ...)           // 65 = MAX_BRANCHES + 1
(0..65u16)                     // Same violation

// Lines 941-947 — boundary values should be named constants
slot_from_text("65535", 0, "test.field")  // u16::MAX - hardcoded
slot_from_text("65536", 0, "test.field")  // u16::MAX + 1 - hardcoded
```

### 3.3 Raw `i64` for Slot Values

```rust
// VIOLATION: Lines 968-969, 1058-1059
crate::CompileError::SlotIndexOutOfRange { value }
if *value == 65536i64          // Raw i64 instead of typed constant
if *value == -1i64            // Raw i64 instead of typed constant
```

### 3.4 Raw String Field Paths

```rust
// VIOLATION: Lines 1019, 1096-1097
slot_from_text("", 3, "choose.branches[].when")  // Raw &str field paths
lower_choose_fanout_exceeds_limit() {
    primitive: "choose",       // Should use a constant
    field: "branches",         // Should use a constant
    value: 65,                 // Already flagged above
    limit: 64,                 // Already flagged above
}
```

### 3.5 Raw YAML Byte Construction

```rust
// VIOLATION: Lines 51-68 — YAML construction is string manipulation
format!(r#"version: velvet-ballistics/v1
name: test
when:
  manual: {{}}
steps:
  - id: collect_step
    collect:
      variable: "{variable}"
      source: "{source}"
      {pages_str}
      {items_str}{body_content}
  - id: done
    finish:
      result: 0
"#).into_bytes()
```

This should use a domain factory or test builder pattern.

---

## 4. SCOTT WLASCHIN DDD VIOLATIONS

### 4.1 No Value Objects — Raw Struct Construction

Every test constructs `StepAst` via raw struct literals:

```rust
// VIOLATION: Lines 409-421, 491-503, 769-783
vb_yaml::ast::StepAst {
    id: "step_a".to_string(),
    name: None,
    condition: None,
    primitive: StepPrimitive::Set {
        output: "a".to_string(),
        value: "1".to_string(),
    },
    with: None,
    retry: None,
    on_error: None,
    then: None,
}
```

**Should have**: Factory functions like `StepAst::set_step(id, output, value)` or a test-only builder.

### 4.2 Repeated Helper Duplication

The `choose_body_set_step` helper (lines 526-540) is repeated inline in many tests because there's no shared test utility module:

```rust
// VIOLATION: Same helper redefined at lines 526-540 and inline at 409-421
fn choose_body_set_step(id: &str, value: &str) -> vb_yaml::ast::StepAst { ... }
```

### 4.3 No Workflow State Transition Modeling

Tests for `lower_canonical_choose` test a **workflow** (YAML → compiled IR) but treat it as a black box with no state machine modeling. The test at line 1333 (`lower_canonical_choose_empty_branches_without_otherwise_returns_empty_branch_table_error`) should model the state transitions explicitly.

---

## 5. REQUIRED SPLIT

The file MUST be split into at least **5 modules** under `mod_compile_lowering/tests/`:

```
mod_compile_lowering/tests/
├── mod.rs                    # Re-exports all test submodules
├── collect_digest_tests.rs   # Lines 1–250: Collect digest coverage
├── digest_primitive_tests.rs # Lines 258–515: Direct digest_step_primitive
├── choose_width_tests.rs     # Lines 520–755: Width & overflow
├── slot_parsing_tests.rs     # Lines 759–1067: slot_from_text
└── choose_lowering_tests.rs  # Lines 1070–1410: Lower/compile behavior
```

Each new file: **< 300 lines** (current largest chunk is ~340 lines).

---

## 6. PRESCRIPTIVE REMEDIATION

### 6.1 Add Named Constants

```rust
// In mod_compile_lowering/tests/mod.rs or a shared test constants module
const MAX_CHOOSE_BRANCHES: u16 = 64;
const MAX_SLOT_INDEX: u32 = u16::MAX as u32;
const BODY_VAR_FIELD: &str = "collect.body";
const CHOOSE_BRANCHES_FIELD: &str = "choose.branches[].when";
```

### 6.2 Create Test Value Object Factories

```rust
// In mod_compile_lowering/tests/mod.rs
pub(crate) fn make_collect_ast(
    variable: impl Into<VariableName>,
    source: impl Into<SourceName>,
    pages: Option<PageCount>,
    items: Option<ItemCount>,
    body: Vec<StepAst>,
) -> StepPrimitive { ... }

pub(crate) fn make_set_step(id: &str, output: &str, value: &str) -> StepAst { ... }
```

### 6.3 Update `mod.rs`

```rust
#[cfg(test)]
mod collect_digest_tests;
#[cfg(test)]
mod digest_primitive_tests;
#[cfg(test)]
mod choose_width_tests;
#[cfg(test)]
mod slot_parsing_tests;
#[cfg(test)]
mod choose_lowering_tests;
```

---

## 7. SUMMARY

| Issue | Severity | Count |
|-------|----------|-------|
| Lines over 300 limit | **CRITICAL** | 1410 / 300 |
| Primitive obsession (`&str`, `u32`, `i64`) | **HIGH** | 15+ instances |
| Raw struct construction instead of factories | **HIGH** | 20+ instances |
| Magic numbers without named constants | **MEDIUM** | 8+ instances |
| Multiple responsibilities in one file | **HIGH** | 6 suites |

**IMMEDIATE ACTION**: Split into 5 test modules. Extract shared helpers to `tests/mod.rs`. Add named constants for domain limits.

---

*Report generated by: architectural-drift agent*
*Workspace: arch-drift-hammer (JJ)*
