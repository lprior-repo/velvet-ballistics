# ARCHITECTURAL DRIFT REPORT: schema.rs
## Target: `/home/lewis/src/velvet-ballistics/crates/vb_validate/src/schema.rs`
## Severity: **CATASTROPHIC** (2195 lines, 631% of 300-line limit)

---

## EXECUTIVE SUMMARY

`schema.rs` is a **2,195-line monolith** that violates every architectural principle in the Scott Wlaschin DDD canon and the <300-line file size mandate. It crams domain types, validation logic, and 1,744 lines of tests into a single undifferentiated mass. The file is structurally incapable of expressing the domain model correctly.

---

## VIOLATION MATRIX

| # | Category | Severity | Lines | Issue |
|---|----------|----------|-------|-------|
| 1 | File Size | **CRITICAL** | 2195/300 | 631% over limit |
| 2 | Schema/Validation Coupling | **CRITICAL** | 336-446 + 75-330 | Types and validators in same file |
| 3 | Primitive Obsession: ID | **HIGH** | 309-326 | `&str` for IDs, manual regex |
| 4 | Primitive Obsession: Version | **HIGH** | 9, 113-123 | `&str` for version constant |
| 5 | Primitive Obsession: ReservedIds | **HIGH** | 43-73 | Raw `&[&str]` for reserved words |
| 6 | Primitive Obsession: Field Names | **HIGH** | 11-41 | `&[&str]` constants instead of types |
| 7 | Primitive Obsession: Trigger Kind | **HIGH** | 147-155 | String matching for trigger variants |
| 8 | Primitive Obsession: Step Primitives | **HIGH** | 38-41, 266-279 | String matching for step types |
| 9 | Test Mass | **HIGH** | 452-2195 | 1,744 lines of inline tests |
| 10 | No Value Objects | **HIGH** | N/A | No `WorkflowId`, `StepId`, `Version`, `TriggerKind`, `StepPrimitive` types |
| 11 | No Aggregate Boundary | **MEDIUM** | 337-359 | `WorkflowDoc` is a dumb container |

---

## DETAILED FINDINGS

### 1. FILE SIZE: 2195 lines (LIMIT: 300)

The file is **7.3x the permitted size**. This is not a large file problem—it is a structural decomposition failure. The file cannot be reviewed, tested, or understood as a coherent unit.

**Required Action:** Split into minimum 5 files:
- `types.rs` — Domain types only
- `id.rs` — `WorkflowId`/`StepId` value objects
- `trigger.rs` — `TriggerKind` and validation
- `step.rs` — `StepPrimitive` and step validation
- `schema.rs` — Orchestration only, delegates to submodules
- `schema_test.rs` — Extracted tests

---

### 2. SCHEMA TYPE / VALIDATION COUPLING (Lines 75-330 + 336-446)

**Current Structure:**
```
schema.rs (2195 lines)
├── Constants (9-73)
├── Validation Functions (75-330)
├── Domain Types (336-446)
└── Tests (452-2195)
```

**Problem:** `WorkflowDoc`, `StepDoc`, and `FieldValue` are defined alongside the functions that validate them. In Scott Wlaschin DDD, a schema type is **data**, and validation is a **separate concern** that produces an `Either<Error, Validated<T>>` result.

**Evidence:**
- `WorkflowDoc::from_pairs` (line 364) constructs from raw `Vec<(String, FieldValue)>`
- `validate_workflow_schema` (line 79) consumes `&WorkflowDoc`
- Both live in the same file with no module boundary between them

**Required Action:** Move domain types to `vb_validate::schema::types` (or a separate crate). Validation functions belong in `vb_validate::validators`.

---

### 3. PRIMITIVE OBSESSION: IDENTITY (`is_valid_id`, lines 309-330)

```rust
fn is_valid_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 64 { return false; }
    let first = id.as_bytes().first();
    // ...manual byte inspection...
}
```

**Problem:** `id: &str` is a raw string. The validation rules (length 1-64, lowercase ASCII + underscore, must start with lowercase) are applied via boolean flag brute force. There is no:
- `WorkflowId` newtype wrapping the validated identifier
- `StepId` newtype  
- Encapsulated validation in a constructor (e.g., `StepId::new(input: &str) -> Result<StepId, InvalidId>`)

**Scott Wlaschin Principle Violated:** "Make illegal states unrepresentable." The raw `&str` can represent any string, including invalid IDs. The type system should rule out invalid IDs at compile time.

**Required Action:**
```rust
// vb_validate::schema::id
pub struct StepId(NonEmpty<MaxLen<AsciiLowercase>>);

impl StepId {
    pub fn new(s: &str) -> ValidationResult<Self> { ... }
}
```

---

### 4. PRIMITIVE OBSESSION: VERSION (lines 9, 113-123)

```rust
const CANONICAL_VERSION: &str = "velvet-ballistics/v1";

pub fn validate_version(doc: &WorkflowDoc) -> ValidationResult<()> {
    match doc.get_string("version") {
        Some(version) if version == CANONICAL_VERSION => Ok(()),
        Some(version) => Err(ValidationError::InvalidVersion { ... }),
        None => Err(ValidationError::MissingRequiredField { ... }),
    }
}
```

**Problem:** Version is a string constant compared via equality. No `Version` type that:
- Parses `velvet-ballistics/v{N}` format
- Validates the dialect name
- Extracts the version number
- Provides structural comparison

**Required Action:** Create `Version` value object.

---

### 5. PRIMITIVE OBSESSION: RESERVED IDS (lines 43-73)

```rust
const RESERVED_IDS: &[&str] = &[
    "now", "random", "runtime", "null", "true", "false",
    "input", "inputs", "vars", "secrets", "steps", "error",
    "attempt", "total", "result", "when", "item", "do",
    "set", "choose", "for_each", "collect", "repeat", "wait",
    "ask", "try_again", "on_error", "then", "finish",
];
```

**Problem:** Reserved IDs are a **set of strings** rather than a **typed set of `ReservedId` values**. This allows typos in the constant to go undetected, and the membership check `RESERVED_IDS.contains(&id)` is O(n) linear scan.

**Required Action:** Replace with `ReservedIdSet` as a `HashSet<StepId>` or `InlineSet<StepId, 32>`.

---

### 6. PRIMITIVE OBSESSION: FIELD NAMES (lines 11-41)

```rust
const ALLOWED_TOP_LEVEL_FIELDS: &[&str] = &[
    "version", "name", "when", "inputs", "vars", "secrets", "result", "examples", "steps",
];

const ALLOWED_STEP_FIELDS: &[&str] = &[
    "id", "name", "if", "with", "then", "set", "choose", "for_each",
    "together", "collect", "reduce", "repeat", "wait", "ask", "finish",
    "do", "on_error", "try_again",
];

const STEP_PRIMITIVES: &[&str] = &[
    "set", "do", "choose", "for_each", "together", "collect", "reduce",
    "repeat", "wait", "ask", "finish",
];
```

**Problem:** Field names are raw strings throughout. There is no:
- `TopLevelField` enum
- `StepField` enum  
- `StepPrimitive` enum
- Type-safe field extraction

**Evidence of coupling:** `validate_unknown_fields` (line 247) iterates `doc.field_names()` and does `ALLOWED_TOP_LEVEL_FIELDS.contains(&field)`. This is string-level coupling everywhere.

**Required Action:** Define enums for field categories. `FieldValue::String(String)` is still raw string.

---

### 7. PRIMITIVE OBSESSION: TRIGGER KIND (lines 147-155)

```rust
match kind.as_str() {
    "manual" | "webhook" => validate_empty_trigger(kind, body),
    "schedule" => validate_named_string_trigger(kind, body, "cron"),
    "event" => validate_named_string_trigger(kind, body, "type"),
    "http" => Err(ValidationError::HttpTriggerOutOfCore),
    other => Err(ValidationError::UnsupportedTrigger { trigger: other.to_owned() }),
}
```

**Problem:** Trigger kind is a `&str` matched via string literals. No `TriggerKind` enum with structured variants like `Manual`, `Webhook`, `Schedule { cron: CronExpr }`, `Event { event_type: EventType }`.

**Required Action:** Define `TriggerKind` enum with variants.

---

### 8. PRIMITIVE OBSESSION: STEP PRIMITIVES (lines 38-41, 266-279)

```rust
pub fn validate_single_primitive(step: &StepDoc) -> ValidationResult<()> {
    let mut count = 0_usize;
    for (field, _) in &step.fields {
        if STEP_PRIMITIVES.contains(&field.as_str()) { count += 1; }
    }
    if count == 0 { return Err(ValidationError::MissingStepPrimitive); }
    if count > 1 { return Err(ValidationError::MultipleStepPrimitives); }
    Ok(())
}
```

**Problem:** Step primitive detection is string matching against `STEP_PRIMITIVES` array. The result is not a typed `StepPrimitive` enum value—it's a side effect counted by iterating fields.

**Required Action:** Replace with `Step::primitive(&self) -> Option<StepPrimitive>` accessor returning `Option<StepPrimitive>`.

---

### 9. TEST MASS (1,744 lines, lines 452-2195)

The tests are **79% of the file** (1,744 of 2,195 lines). This is a structural disaster:

- Tests should be in `schema_test.rs` or `tests/schema.rs` (integration tests)
- Unit tests can remain in-module but should be <20% of production code
- Current ratio: 2.9x more test code than production code in the same file

**Required Action:** Extract to `tests/schema_prop.rs` (property tests), `tests/schema_bdd.rs` (BDD tests). Inline unit tests should cover only happy paths; adversarial tests belong in the test harness.

---

### 10. NO VALUE OBJECTS

The file defines zero value objects. The domain model is entirely composed of:

| Concept | Representation | Problem |
|---------|---------------|---------|
| `WorkflowId` | None (uses `name: &str` field) | No type |
| `StepId` | None (raw `&str`) | Primitive obsession |
| `Version` | None (raw `&str`) | Primitive obsession |
| `TriggerKind` | None (raw `&str` matching) | Primitive obsession |
| `StepPrimitive` | None (raw `&str` matching) | Primitive obsession |
| `CronExpr` | None (raw `&str`) | Not validated |
| `EventType` | None (raw `&str`) | Not validated |
| `ReservedId` | None (`&[&str]` flat list) | No type |

---

### 11. NO AGGREGATE BOUNDARY

`WorkflowDoc` is a dumb `Vec<(String, FieldValue)>` wrapper. It has no:
- Invariants enforced at construction
- `StepDoc` children that are validated as part of workflow aggregate
- Consistency boundary (duplicate field detection is external)

In DDD, the `Workflow` aggregate root should:
1. Validate step IDs are unique within the workflow
2. Validate step references in `then`, `on_error`, etc. point to existing steps
3. Validate `for_each`/`reduce` step variable bindings are in scope

None of this is expressible with `WorkflowDoc`.

---

## PRESCRIPTION

### Minimum Viable Refactor (5 files)

```
vb_validate/src/
├── schema/
│   ├── mod.rs          # Re-exports, delegates to submodules
│   ├── types.rs        # WorkflowDoc, StepDoc, FieldValue (110 lines)
│   ├── id.rs           # WorkflowId, StepId value objects (80 lines)
│   ├── trigger.rs      # TriggerKind enum + validation (90 lines)
│   └── step.rs         # StepPrimitive enum + validation (90 lines)
├── validators.rs       # All validate_* functions (260 lines)
└── lib.rs              # Re-exports
```

**Lines after refactor:**
- `types.rs`: ~110 lines (domain types only)
- `id.rs`: ~80 lines (ID value objects with validation)
- `trigger.rs`: ~90 lines (trigger type + validation)
- `step.rs`: ~90 lines (step primitive type + validation)
- `validators.rs`: ~260 lines (orchestration, delegates to types)
- `mod.rs`: ~50 lines (orchestration, delegation)

**Total: ~680 lines** (still needs further split of validators)

### Ideal State (9+ files)

Each validator function group and its associated types should be in their own module.

---

## ENFORCEMENT GATE

Before this file can be considered "drift-corrected," the following must be true:

- [ ] File size < 300 lines
- [ ] `WorkflowId` and `StepId` newtype types exist with `TryFrom<&str>` implementations
- [ ] `Version` value object exists with `TryFrom<&str>`
- [ ] `TriggerKind` enum exists with variants: `Manual`, `Webhook`, `Schedule(CronExpr)`, `Event(EventType)`
- [ ] `StepPrimitive` enum exists with 11 variants
- [ ] `FieldValue::String(String)` replaced with typed value fields
- [ ] `RESERVED_IDS` replaced with `ReservedIdSet`
- [ ] Tests moved to `tests/schema_bdd.rs` and `tests/schema_prop.rs`
- [ ] No `&str` used for ID, version, or trigger kind in function signatures outside the value-object constructors

---

## VERDICT

**CLASSIFICATION:** Structural Collapse  
**PRIORITY:** P0 — Immediate refactor required  
**ESTIMATED BREAKAGE RISK:** Low (validation behavior is well-tested, refactor is purely structural)  
**ESTIMATED REFACTOR COST:** 2-3 days for proper type decomposition

This file is not "large"—it is architecturally incoherent. The 1,744 lines of tests are evidence that the domain model is not expressed in types, requiring exhaustive test coverage to compensate. Proper type design would reduce test surface area by >60%.

---
*Report generated by arch-drift-hammer on 2026-05-29*
*Drift Agent: architectural-drift v1*
