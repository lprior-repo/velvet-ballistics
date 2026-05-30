# ARCHITECTURAL DRIFT HAMMER REPORT
## Target: `crates/vb_validate/src/schema_fields.rs`
## Line Count: **805** — EXCEEDS 300-LINE LIMIT BY **168%**

---

## VERDICT: REFACTOR REQUIRED

---

## 1. LINE COUNT VIOLATION

| File | Lines | Limit | Excess |
|------|-------|-------|--------|
| `schema_fields.rs` | 805 | 300 | +168% |

**This file is 2.68× the size limit.** It MUST be split.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Raw `String` for Domain Identifiers

Every `ValidationError` construction uses `.to_owned()` on string literals:

```rust
// VIOLATION — "name", "version", "step id" are raw strings
Err(ValidationError::MissingRequiredField {
    field: "version".to_owned(),   // should be FieldName type
})
Err(ValidationError::MissingRequiredField {
    field: "step id".to_owned(),   // should be FieldName type
})
Err(ValidationError::InvalidId {
    id: format!("{field}: {id}"),  // should be StepId type
})
```

**DDD Rule:** `String` should never appear in domain roles. Use NewTypes:
- `WorkflowName` wrapping `String`
- `StepId` wrapping `String`
- `FieldName` wrapping `String`
- `Version` wrapping `String`

### 2.2 `&[&str]` Slices for Field Name Sets

```rust
const ALLOWED_TOP_LEVEL_FIELDS: &[&str] = &[...];   // PRIMITIVE OBSESSION
const ALLOWED_STEP_FIELDS: &[&str] = &[...];
const STEP_PRIMITIVES: &[&str] = &[...];
```

These string sets should be `EnumSet<FieldKind>` or a proper `FieldRegistry` type.
Every loop does `.contains(&field.as_str())` on raw strings — zero type safety.

### 2.3 Raw String Comparison for Trigger Kind

```rust
match kind.as_str() {
    "manual" | "webhook" => validate_empty_trigger(kind, body),
    "schedule" => validate_named_string_trigger(kind, body, "cron"),
    "event" => validate_named_string_trigger(kind, body, "name"),
    "http" => Err(ValidationError::HttpTriggerOutOfCore),
    other => Err(ValidationError::UnsupportedTrigger { trigger: other.to_owned() }),
}
```

**VIOLATION:** `kind` is a raw `&str` being pattern-matched on string literals.
This should be `enum TriggerKind { Manual, Webhook, Schedule, Event, Http }`.

### 2.4 `String` for Error Messages

`ValidationError` variants like `InvalidVersion { version: String }`, `UnsupportedTrigger { trigger: String }` carry raw strings. These should use typed error codes or structured error domains.

---

## 3. SCOTT WLASCHIN DDD VIOLATIONS

### 3.1 Workflow — No Explicit State Transition

`validate_workflow_schema` chains 6 independent validation functions:

```rust
pub fn validate_workflow_schema(doc: &WorkflowDoc) -> ValidationResult<()> {
    validate_duplicate_fields(doc)?;  // state: dirty
    validate_required_fields(doc)?;   // state: dirty
    validate_unknown_fields(doc)?;    // state: dirty
    validate_version(doc)?;            // state: checked
    validate_trigger(doc)?;            // state: checked
    validate_ids(doc)?;                // state: checked
    Ok(())
}
```

This is **validation-as-procedure**, not a workflow state machine. Per Wlaschin, a workflow should have explicit states and transitions. The validation order implies a state progression but it's not modeled as such.

### 3.2 No Value Objects for Schema Primitives

The entire domain layer uses raw `String` instead of Value Objects:
- No `Version` — just `"velvet-ballistics/v1"` constant
- No `StepId` — just `&str` validated by regex
- No `FieldName` — just `&str` in string sets
- No `TriggerKind` enum — just `&str` pattern matching

**Rule:** Make illegal states unrepresentable. A `StepId` newtype can only be constructed via `StepId::parse(id)` which returns `Result<StepId, InvalidIdError>`. The raw string can't accidentally appear where a validated ID is required.

### 3.3 Primitive Sets Instead of Domain Sets

`ALLOWED_TOP_LEVEL_FIELDS`, `ALLOWED_STEP_FIELDS`, `STEP_PRIMITIVES` are all `&[&str]`. They should be proper set types:
```rust
struct StepFieldRegistry { fields: EnumSet<StepField> }
enum StepField { Id, Name, If, With, Then, Set, Choose, ForEach, ... }
```

### 3.4 O(n²) Duplicate Detection

```rust
fn validate_no_duplicate_names(fields: &[(String, FieldValue)]) -> ValidationResult<()> {
    let mut seen: Vec<&str> = Vec::with_capacity(fields.len());
    for (name, _) in fields {
        if seen.contains(&name.as_str()) {  // O(n) per iteration = O(n²)
            return Err(ValidationError::DuplicateKey);
        }
        seen.push(name.as_str());
    }
    Ok(())
}
```

Should use `HashSet<&str>` or `Vec` with `binary_search`. This is both a perf issue and a symptom of treating primitives as primitives.

---

## 4. COHESION VIOLATIONS

This file mixes 4 distinct validation domains:

| Domain | Functions |
|--------|-----------|
| Top-level schema validation | `validate_workflow_schema`, `validate_required_fields`, `validate_unknown_fields`, `validate_duplicate_fields` |
| Version validation | `validate_version` |
| Trigger validation | `validate_trigger`, `validate_empty_trigger`, `validate_named_string_trigger` |
| Step validation | `validate_step_fields`, `validate_step_unknown_fields`, `validate_single_primitive` |
| ID validation | `validate_ids`, `validate_id` |

These should be in separate modules:
```
schema/
  schema_fields.rs      — Top-level + version (current ~80 lines)
  schema_trigger.rs     — Trigger validation (~70 lines)
  schema_step.rs        — Step field + primitive validation (~60 lines)
  schema_id.rs          — Already exists but tightly coupled
```

---

## 5. PROPOSED SPLIT PLAN

### Split 1: `schema_version.rs` (~80 lines)
- `validate_version`
- `CANONICAL_VERSION` constant
- `validate_required_fields`
- `validate_unknown_fields`
- `validate_duplicate_fields`
- `validate_no_duplicate_names`

### Split 2: `schema_trigger.rs` (~70 lines)
- `validate_trigger`
- `validate_empty_trigger`
- `validate_named_string_trigger`
- TriggerKind enum (NEW — replaces string pattern matching)

### Split 3: `schema_step.rs` (~70 lines)
- `validate_step_fields`
- `validate_step_unknown_fields`
- `validate_single_primitive`
- `ALLOWED_STEP_FIELDS`, `STEP_PRIMITIVES` (as proper enum sets)

### Split 4: `schema_ids.rs` (~30 lines)
- `validate_ids`
- `validate_id`
- Already has companion `schema_id.rs` — strengthen with newtypes

### Remaining: `schema_fields.rs` (~40 lines)
- `validate_workflow_schema` — orchestration only
- Re-exports from split modules
- No business logic

---

## 6. REQUIRED NEWTYPE DEFINITIONS

```rust
// Newtypes to replace raw String in domain roles
pub struct Version(String);
pub struct WorkflowName(String);
pub struct StepId(String);
pub struct FieldName(String);

impl FieldName {
    pub fn from_str(s: &str) -> Self { FieldName(s.to_owned()) }
}

impl StepId {
    pub fn parse(s: &str) -> Result<Self, InvalidIdError> {
        // existing is_valid_id logic
    }
}
```

---

## 7. SUMMARY

| Issue | Severity |
|-------|----------|
| 805 lines (limit: 300) | **CRITICAL** |
| Raw `String` for all domain identifiers | **CRITICAL** |
| `&[&str]` for field registries | **HIGH** |
| String pattern matching for `TriggerKind` | **HIGH** |
| O(n²) duplicate detection | **MEDIUM** |
| No explicit workflow state transitions | **MEDIUM** |
| 5 validation domains in one file | **HIGH** |

**STATUS: REFACTOR REQUIRED**

The file MUST be split into at least 4 modules. Newtypes MUST replace raw strings. TriggerKind MUST become an enum. The test module (545 lines of tests) should remain with the top-level orchestration or move to integration tests.
