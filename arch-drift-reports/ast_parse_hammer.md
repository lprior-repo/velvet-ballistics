# Architectural Drift Report: `ast/parse.rs`

**File:** `crates/vb_compile/src/ast/parse.rs`  
**Line Count:** 724  
**Limit:** 300  
**Status:** 🔴 CRITICAL VIOLATION — 2.41x over limit

---

## 1. Line Count Violation

| Metric | Value |
|--------|-------|
| Actual lines | 724 |
| Limit | 300 |
| Over by | 424 lines |
| Ratio | 2.41x |

**Mandatory split required.** The file must be decomposed into focused modules.

---

## 2. Primitive Obsession Violations (Scott Wlaschin DDD)

### 2.1 Raw `usize` for Step Indices

**Location:** Throughout, e.g. lines 241, 247, 251, 264

**Problem:**
```rust
fn parse_step(step: &Yaml<'_>, index: usize, marks: &AstMarks) -> Result<StepAst, CompileError>
fn parse_step_kind(mapping: &saphyr::Mapping<'_>, index: usize, ...) -> ...
fn primitive_entry<'map, 'input>(mapping: &'map saphyr::Mapping<'input>, index: usize, ...) -> ...
```

`index: usize` is raw primitive obsession. While `StepIdx` newtype exists downstream, the **parser intermediate** uses raw `usize` everywhere. This means invalid step indices can exist in the parsing layer without being caught by a type.

**Fix:** Introduce `StepIndex(input: usize)` newtype wrapping the raw index, with a `from_yaml(node: &Yaml, field: &str) -> Result<StepIndex>` constructor that validates bounds at parse time.

### 2.2 Raw `&'static str` for Field Names

**Locations:** Lines 29, 39, 52, 143, 154, 203, 477, 492, 542, 558, 581

**Problem:**
```rust
fn required_str<'a>(doc: &'a Yaml<'a>, field: &'static str) -> Result<&'a str, CompileError>
fn required_mapping<'a>(doc: &'a Yaml<'a>, field: &'static str) -> ...
fn required_sequence<'a>(doc: &'a Yaml<'a>, field: &'static str) -> ...
fn parse_value_map(doc: &Yaml<'_>, field: &'static str, ...) -> ...
```

`field: &'static str` is primitive obsession. Field names like `"version"`, `"name"`, `"steps"`, `"inputs"` are repeated as raw strings throughout.

**Fix:** Define a `FieldName` newtype:
```rust
pub struct FieldName(Box<str>);
impl FieldName {
    pub const VERSION: FieldName = FieldName(Box::from("version"));
    pub const NAME: FieldName = FieldName(Box::from("name"));
    pub const STEPS: FieldName = FieldName(Box::from("steps"));
    pub const INPUTS: FieldName = FieldName(Box::from("inputs"));
    // etc.
}
```

### 2.3 Raw `i64` for YAML Integer Values

**Locations:** Lines 448, 450, 502, 516, 574-578, 593, 610

**Problem:**
```rust
fn finish_integer_is_slot(value: i64, index: usize) -> bool
fn parse_slot_expr(value: i64) -> Result<AstExpression, CompileError>
fn parse_step_idx(node: &Yaml<'_>) -> Result<StepIdx, CompileError> {
    let value = node.as_integer().ok_or(...)?; // returns i64
    let raw = u16::try_from(value).map_err(...)?; // unchecked i64->u16
}
```

Raw `i64` leaks through the entire parsing stack. Negative integers, overflow values (e.g. `i64::MAX` passed as step index) are not caught until mid-parse.

**Fix:** Create `YamlInteger(i64)` newtype with bounded constructors:
```rust
pub struct YamlInteger(i64);
impl YamlInteger {
    pub fn try_into_u16(self) -> Result<u16, CompileError> { ... }
    pub fn try_into_slot_idx(self, step: usize) -> Result<SlotIdx, CompileError> { ... }
}
```

### 2.4 Raw `&str` for Trigger Kind

**Location:** Lines 77, 79-87

**Problem:**
```rust
let kind = key.as_str().ok_or_else(crate::non_string_key_error)?;
match kind {
    "manual" => ...
    "webhook" => ...
    "schedule" => ...
    "event" => ...
    other => Err(CompileError::UnknownTriggerKind { trigger: other.into() }),
}
```

`&str` for trigger kind with stringly-typed matching is classic primitive obsession. No exhaustiveness guarantee at compile time.

**Fix:** Define `TriggerKind` enum:
```rust
pub enum TriggerKind { Manual, Webhook, Schedule, Event }
impl TriggerKind {
    pub fn from_str(s: &str) -> Option<TriggerKind> { ... }
}
```

### 2.5 Raw `&str` for Step Primitive Names

**Location:** Lines 269-287, 307-324

**Problem:**
```rust
match field {
    "set" => parse_save(body).map(|kind| (StepPrimitiveAst::Set, kind)),
    "run" => parse_run(body, index).map(|kind| (StepPrimitiveAst::Run, kind)),
    // 11 more string matches...
    _ => Err(CompileError::UnknownStepField { step: index, field: field.into() }),
}
```

And helper:
```rust
fn is_supported_primitive(field: &str) -> bool {
    matches!(field, "set" | "run" | "do" | "save" | "choose" | "for_each" | "parallel" | "collect" | "aggregate" | "repeat" | "wait" | "ask" | "finish")
}
```

**Fix:** `StepPrimitive` enum with `from_str` and exhaustive matching.

---

## 3. Parse, Don't Validate Assessment

**Verdict: PARTIAL COMPLIANCE**

The file does use `Result<T, CompileError>` throughout and constructs domain types directly. However:

✅ **Good:**
- `parse_workflow_ast` returns `WorkflowAst` directly (no validation layer after)
- `StepIdx::new(raw)` / `SlotIdx::new(raw)` are called at parse boundaries
- `AstExpression::Slot(SlotIdx::new(raw))` is constructed correctly

❌ **Bad:**
- Raw `i64` parsing means invalid integers (negative, out-of-range) can appear mid-parse before bounds checking
- `finish_integer_is_slot(value: i64, index: usize)` — the validation logic for whether an integer is a slot reference is hand-written inline rather than being part of a `YamlInteger` type's constructor logic
- `integer_error_value(value: i64) -> usize` (lines 574-578) is a pure-integer utility that exists outside any domain type

---

## 4. Recommended Module Split

| Module | Responsibility | Est. Lines |
|--------|---------------|------------|
| `ast/parse/mod.rs` | Entry point `parse_workflow_ast`, `required_*` helpers, `parse_map` | ~180 |
| `ast/parse/trigger.rs` | `parse_trigger`, `parse_*_trigger`, `trigger_str` | ~80 |
| `ast/parse/steps.rs` | `parse_steps`, `parse_step`, `parse_step_kind`, all `parse_*` fns | ~200 |
| `ast/parse/values.rs` | `parse_value`, `parse_non_scalar_value`, `text_or_ref` | ~80 |
| `ast/parse/shared.rs` | `StepIndex`, `YamlInteger`, `FieldName` newtypes + `FromYaml` trait | ~100 |
| `ast/parse/tests.rs` | Tests | ~70 |

---

## 5. Action Items

1. **[MANDATORY]** Split file into 5 modules under `ast/parse/`
2. **[MANDATORY]** Introduce `StepIndex(usize)` newtype for step indices
3. **[MANDATORY]** Introduce `YamlInteger(i64)` with bounded constructors  
4. **[MANDATORY]** Introduce `FieldName` const-based newtype
5. **[MANDATORY]** Define `TriggerKind` enum with `FromStr` impl
6. **[MANDATORY]** Define `StepPrimitive` enum with `FromStr` impl
7. **[REVIEW]** Move `integer_error_value` into `YamlInteger` as `error_value(&self) -> usize`

---

## 6. Files to Create/Modify

```
crates/vb_compile/src/ast/parse/
├── mod.rs      (NEW - entry point, ~180 lines)
├── trigger.rs  (NEW - trigger parsing, ~80 lines)
├── steps.rs    (NEW - step parsing, ~200 lines)
├── values.rs   (NEW - value parsing, ~80 lines)
├── shared.rs   (NEW - newtypes: StepIndex, YamlInteger, FieldName, ~100 lines)
└── tests.rs    (NEW - tests moved from original, ~70 lines)

crates/vb_compile/src/ast/mod.rs (MODIFY - add `pub mod parse;` and submodules)
```

---

**STATUS:** 🔴 DRIFT DETECTED — MANDATORY REFACTOR REQUIRED

File exceeds line limit by 2.41x and contains pervasive primitive obsession that violates Scott Wlaschin DDD principles. The "Parse, don't validate" pattern is undercut by raw integer primitives flowing through the parsing layer unchecked until mid-function bounds checks.
