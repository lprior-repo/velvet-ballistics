# Architectural Drift Report: `vb_yaml` AST Parse Module

**Target:** `/home/lewis/src/velvet-ballistics/crates/vb_yaml/src/ast/parse*`
**Report Date:** 2026-05-29
**Enforcer:** arch-drift-hammer
**Status:** ⚠️ MULTIPLE VIOLATIONS

---

## Executive Summary

The parse module comprises 4 files totaling **839 lines** of production parse logic. **Two files violate the <300 line rule**: `parse_steps.rs` (354 lines) and `types.rs` (413 lines). Additionally, pervasive **primitive obsession** infects both types and parsing helpers, with raw `String`, `u16`, `u32`, and `i64` used where domain types should be employed.

---

## §1 — Size Violations (MANDATORY <300 LINE RULE)

| File | Lines | Status |
|------|-------|--------|
| `parse.rs` | 196 | ✅ PASS |
| `parse_fields.rs` | 193 | ✅ PASS |
| `parse_steps.rs` | **354** | ❌ FAIL — exceeds by 54 |
| `parse_trigger.rs` | 96 | ✅ PASS |
| `types.rs` | **413** | ❌ FAIL — exceeds by 113 |

**Total parse module (excluding mod.rs):** 839 lines across 4 files. Target would be ≤1200 if split correctly.

---

## §2 — Primitive Obsession Violations (Scott Wlaschin DDD)

### 2.1 Raw Strings Everywhere

The following fields use `String` where a **typed value object** should exist:

| Location | Field | Problem |
|----------|-------|---------|
| `TriggerAst::Schedule` | `cron: String` | Cron expression is unparsed/unvalidated raw string |
| `TriggerAst::Event` | `event_type: String` | Event name is untyped |
| `StepAst::id` | `id: String` | Step identifier is unvalidated raw string |
| `StepAst::condition` | `condition: Option<String>` | Expression language is untyped |
| `StepAst::name` | `name: Option<String>` | No validation |
| `StepAst::with` | `with: Option<String>` | No validation |
| `StepAst::then` | `then: Option<String>` | No validation |
| `StepPrimitive::Set { output, value }` | Both `String` | Variable name and expression untyped |
| `StepPrimitive::Do { action, input }` | Both `String` | Action name untyped |
| `StepPrimitive::Wait { event, timeout }` | Both `Option<String>` | Expressions untyped |
| `StepPrimitive::Ask { prompt, timeout }` | Both `String` | Prompt untyped |
| `RetryPolicy::delay` | `Option<String>` | Duration/expression untyped |
| `ErrorHandlerAst::handler` | `String` | Handler reference untyped |
| `InputField::key` | `String` | Field name untyped |
| `VarField::key` | `String` | Variable name untyped |
| `SecretField::key` | `String` | Secret name untyped |
| `SecretField::value` | `String` | Secret value unvalidated |
| `ChooseBranch::when` | `String` | Branch condition untyped |
| `TogetherBranch::label` | `String` | Label untyped |

**Verdict:** 20+ raw string fields that should be domain types.

### 2.2 Numeric Primitives Without Bounds

| Location | Type | Problem |
|----------|------|---------|
| `RetryPolicy::max_attempts` | `u16` | Maximum value 65535; workflows likely have much lower practical limits |
| `StepPrimitive::Repeat::max_attempts` | `u16` | Same issue |
| `StepPrimitive::ForEach::at_once` | `Option<u32>` | No upper bound enforced |
| `StepPrimitive::Collect::pages` | `Option<u32>` | No upper bound |
| `StepPrimitive::Collect::items` | `Option<u32>` | No upper bound |
| `AuthorValue::I64` | `i64` | Full range allowed; likely too large for workflow contexts |

**Action required:** Introduce `MaxAttempts(u16)`, `ConcurrencyLimit(u32)`, `PageLimit(u32)` wrapper types that enforce domain-appropriate bounds at construction.

### 2.3 `ScalarValue` Is Underenforced

`ScalarValue` (types.rs:330) is defined as:
```rust
pub enum ScalarValue {
    String(String),
    Integer(i64),
}
```
But `parse_scalar_in` (parse.rs:97) silently accepts `i64` when `String` was expected. The parser **cannot distinguish** between these at parse time — both are "scalar" in YAML. This type adds no type safety over raw YAML.

### 2.4 Missing Type Dividers (Data vs. Calc vs. Action)

Wlaschin DDD demands **Data-Calc-Action** separation. Current structure blurs them:

- **Data types** (should be pure): `InputField`, `VarField`, `SecretField`, `AuthorEntry`, `ExampleAst`
- **Calc types** (should contain domain logic): `AuthorValue` (recursive structure is calc-like, but construction is in parse module)
- **Action types** (workflow execution): `StepPrimitive` variants

The `parse_*` functions in `parse_steps.rs` are **pure parsing** but intermixed with domain concern comments and field-shape validation that belongs in a domain validator.

---

## §3 — Parse Responsibility Map

```
parse_workflow_ast(text: &str) → WorkflowSource
    └── parse_workflow_from_yaml(root: &Yaml)
            ├── reject_unknown_fields (general helper)
            ├── require_str("version")
            ├── require_str("name")
            ├── parse_trigger(root)
            │       ├── parse_when_trigger
            │       │       ├── parse_schedule
            │       │       └── parse_event
            │       └── (manual, webhook are empty-body shortcuts)
            ├── parse_inputs(root)
            ├── parse_vars(root)
            ├── parse_secrets(root)
            ├── parse_steps(root)
            │       ├── parse_step
            │       │   ├── reject_unknown_step_fields
            │       │   ├── require_str_in("id")
            │       │   ├── opt_str("name")
            │       │   ├── opt_str("if")
            │       │   ├── parse_step_primitive
            │       │   │   ├── is_primitive (hardcoded allowlist)
            │       │   │   └── 14 primitive parsers (set, save, do, run, choose, foreach, together, collect, reduce, repeat, wait, ask, finish)
            │       │   ├── opt_str("with")
            │       │   ├── parse_retry
            │       │   ├── parse_error_handler
            │       │   └── opt_str("then")
            │       └── parse_body_steps (recursive)
            ├── parse_result(root)
            └── parse_examples(root)
                    └── parse_author_value (recursive!)
```

**Key observation:** `parse_author_value` is recursive and handles 5 YAML node types. It belongs in a dedicated **value parser** module, not `parse_fields.rs`.

---

## §4 — Hardcoded Magic Strings

### 4.1 `is_primitive()` Allowlist (parse_steps.rs:98-116)
```rust
fn is_primitive(field: &str) -> bool {
    matches!(
        field,
        "set" | "save" | "do" | "run" | "choose" | "foreach" | "for_each"
            | "together" | "collect" | "reduce" | "repeat" | "wait" | "ask" | "finish"
    )
}
```
This allowlist is **not derived from `StepPrimitive` enum variants** — it's a second source of truth that can drift. Should be generated from the enum or at minimum referenced.

### 4.2 `reject_unknown_step_fields` Hardcoded List (parse_steps.rs:118-147)
The list of allowed fields is duplicated between `is_primitive()` allowlist and `reject_unknown_step_fields()`. Any addition to one without the other is a silent bug.

### 4.3 Legacy Rejection Logic In Band (parse_steps.rs:54-65)
```rust
if key == "parallel" { return Err(LegacyPrimitive { primitive: "parallel", canonical: "together" }); }
if key == "aggregate" { return Err(LegacyPrimitive { primitive: "aggregate", canonical: "reduce" }); }
```
These intercept hardcoded strings **before** the `is_primitive()` gate. New legacy mappings require code changes.

---

## §5 — Type System Gaps

### 5.1 No `StepId` Type
Step IDs are used as plain `String`. There's no:
- Naming convention enforcement
- Uniqueness validation (done separately in validation layer)
- Display formatting

### 5.2 No `Version` Type
`version: String` accepts any string. Should be:
```rust
pub struct Version(/* invariant: validated semver or "velvet-ballistics/v1" */);
impl Version {
    pub fn parse(s: &str) -> YamlResult<Self> { ... }
}
```

### 5.3 No `CronExpr` Type
`TriggerAst::Schedule { cron: String }` — cron expressions are unparsed at this stage. Should be a `CronExpr(String)` newtype with at minimum format checking.

### 5.4 No `EventType` Type
`TriggerAst::Event { event_type: String }` — should be `EventType(String)` with format validation.

### 5.5 No `Delay` Type for Retry
`RetryPolicy::delay: Option<String>` — accepts any string. Should be `Delay(String)` or a proper duration type.

---

## §6 — File Organization Issues

### 6.1 `parse_steps.rs` 354 Lines — Must Split

`parse_steps.rs` handles **14 primitive types** plus retry/error handling. Suggested split:
- `parse_steps.rs` — entry point `parse_steps()` only (~20 lines)
- `parse_step_primitives.rs` — all `parse_*` functions for primitives (~200 lines)
- `parse_step_helpers.rs` — `parse_retry`, `parse_error_handler`, `parse_body_steps` (~80 lines)

### 6.2 `types.rs` 413 Lines — Must Split

`types.rs` contains mixed concerns:
- Top-level `WorkflowSource` and parts bundle (114 lines)
- Trigger types (~15 lines)
- Author value types (20 lines)
- Step types (~120 lines)
- Supporting types (~60 lines)
- Feature-gated visibility hacks (40+ lines)

Suggested split:
- `types/workflow.rs` — `WorkflowSource`, `WorkflowSourceParts`
- `types/step.rs` — `StepAst`, `StepPrimitive`, branch types
- `types/value.rs` — `AuthorValue`, `AuthorEntry`, `ScalarValue`
- `types/trigger.rs` — `TriggerAst`
- `types/supporting.rs` — `RetryPolicy`, `ErrorHandlerAst`, `InputField`, `VarField`, `SecretField`, `ResultMapping`, `ExampleAst`

### 6.3 `parse_fields.rs` and `parse_trigger.rs` Are Clean
Both are under 200 lines and focused. `parse_fields.rs` is borderline but acceptable.

---

## §7 — Summary of Required Refactors

| Priority | Issue | Fix |
|----------|-------|-----|
| P0 | `parse_steps.rs` 354 lines | Split into 3 files |
| P0 | `types.rs` 413 lines | Split into 5+ type files |
| P0 | No `StepId` type | Newtype `StepId(String)` with validation |
| P0 | No `Version` type | Newtype `Version(String)` with validation |
| P1 | `ScalarValue` provides no safety | Either enforce at parse time or remove |
| P1 | `max_attempts: u16` unbounded | Newtype `MaxAttempts(u16)` with domain bounds |
| P1 | `is_primitive()` allowlist drift risk | Derive from `StepPrimitive` variants |
| P1 | `cron: String` untyped | Newtype `CronExpr(String)` |
| P1 | `event_type: String` untyped | Newtype `EventType(String)` |
| P2 | `parse_author_value` in parse_fields | Move to dedicated value parser module |
| P2 | Retry/error handling in steps file | Already flagged for extraction |
| P2 | `AuthorEntry<T>` key is `String` | Should be `FieldName(String)` |

---

## §8 — Enforcement Verdict

**ARCHITECTURAL DRIFT: CONFIRMED**

1. **Size violations:** 2 files exceed <300 line hard limit
2. **Primitive obsession:** 20+ raw string fields, 6+ untyped numeric fields, missing domain newtypes
3. **Magic strings:** Allowlist not derived from types; duplicate field lists
4. **File cohesion:** Multiple responsibilities co-located in `parse_steps.rs` and `types.rs`

**Recommended Action:** 
- Immediately split `parse_steps.rs` and `types.rs`
- Introduce `StepId`, `Version`, `CronExpr`, `EventType` newtypes before next release
- Add `MaxAttempts`, `ConcurrencyLimit` bounded types
- Derive `is_primitive()` from `StepPrimitive` at compile time

---

*Report generated by arch-drift-hammer. No files modified — findings only.*
