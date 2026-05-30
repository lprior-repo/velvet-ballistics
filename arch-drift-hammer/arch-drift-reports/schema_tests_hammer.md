# ARCHITECTURAL DRIFT REPORT: schema_tests.rs

**File:** `crates/vb_validate/src/schema_tests.rs`
**Line Count:** 1490 lines (LIMIT: 300)
**Violation Multiplier:** 4.97x over limit
**Status:** 🚨 CRITICAL VIOLATION - MANDATORY SPLIT

---

## 1. LINE COUNT VIOLATION

| Metric | Value |
|--------|-------|
| Actual | 1490 |
| Limit | 300 |
| Over | +1190 |
| Violation | 497% of limit |

**Verdict:** File MUST be split into logical modules.

---

## 2. RESPONSIBILITY MAPPING

The file exercises **5 distinct responsibility clusters**:

### Cluster A: Test Factories (Lines 1–40)
- `make_workflow()` — constructs `WorkflowDoc` from pairs
- `make_step()` — constructs `StepDoc` from pairs
- `valid_workflow_doc()` — canonical valid workflow

### Cluster B: Schema Validation Tests (Lines 42–800)
- Version validation (correct, missing, wrong, empty)
- Trigger validation (manual, schedule, event, webhook, http, ipc)
- ID validation (format, reserved words, duplicates)
- Step field validation (primitives, metadata, legacy fields)
- Workflow schema (unknown fields, duplicate keys)

### Cluster C: Accessor/Query Tests (Lines 806–1006)
- `get_string()`, `get_mapping()`, `get_sequence()`
- `has_field()`, `field_names()`
- Roundtrip `from_pairs()`

### Cluster D: Adversarial BDD Tests (Lines 1012–1490)
- Reserved ID attacks (`input`, `vars`, `secrets`, `steps`, `error`, `attempt`, `result`, `when`, `item`)
- Multiple trigger attacks
- Empty steps attack
- Unknown field attacks
- Step primitive multi-action attacks

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3.1 Field Name Primitives (Stringly-Typed)
```rust
// These are raw &str used throughout — VIOLATION
"version", "name", "steps", "when", "id"
"set", "do", "finish", "choose"
"bogus_field", "payload"
```

**NewType Candidates:**
```rust
struct FieldName(&'static str);
struct StepPrimitive(&'static str);
struct WorkflowField(&'static str);
```

### 3.2 Version String Primitive
```rust
// Raw string literal scattered across ~30 test cases
FieldValue::String("velvet-ballistics/v1".to_owned())
```
**NewType Candidate:**
```rust
struct Version(&'static str);
const CURRENT_VERSION: Version = Version("velvet-ballistics/v1");
```

### 3.3 Trigger Type Primitives
```rust
// Raw strings for trigger kinds
"manual", "schedule", "event", "webhook", "http", "ipc", "cron", "timer"
```
**NewType Candidate:**
```rust
struct TriggerKind(&'static str);
enum Trigger { Manual, Schedule, Event, Webhook }
```

### 3.4 Reserved ID Primitives
```rust
// Hardcoded in adversarial tests AND in schema_id validation
"runtime", "input", "vars", "secrets", "steps", "error", "attempt", "result", "when", "item"
```
**NewType Candidate:**
```rust
struct ReservedId(&'static str);
const RESERVED_IDS: &[ReservedId] = &[...];
```

### 3.5 Error Message Field Primitives
```rust
// In ValidationError variants
field: "name".to_owned(),  // should be FieldName
field: "version".to_owned(),
field: "step id".to_owned(),  // NOT consistent with "steps" above
```

---

## 4. SCOTT WLASCHIN DDD VIOLATIONS

### 4.1 Primitive Obsession on ValidationError
The error enum uses raw strings for field identification:
```rust
ValidationError::MissingRequiredField { field: "name".to_owned() }
```
Should be:
```rust
ValidationError::MissingRequiredField { field: FieldName }
```

### 4.2 Inconsistent Field Naming
- `"step id"` (two words) vs `"steps"` (plural) vs `"step id"` (mixed)
- No consistency in `MissingRequiredField` error construction

### 4.3 No Parse, Don't Validate Discipline
The `make_workflow` and `make_step` helpers accept raw `Vec<(&str, FieldValue)>` rather than a typed builder that guarantees validity.

---

## 5. REQUIRED REFACTORING

### Split into Modules (target: ~250 lines each)

```
schema_tests.rs
├── helpers.rs          # make_workflow, make_step, valid_workflow_doc
├── version_tests.rs    # Version validation tests
├── trigger_tests.rs    # Trigger validation tests  
├── id_tests.rs         # ID validation tests (format, reserved, duplicate)
├── step_tests.rs       # Step field validation tests
├── accessor_tests.rs   # WorkflowDoc/StepDoc query tests
└── adversarial_tests.rs # BDD bypass attack tests
```

### NewType Introduction (schema domain)
```rust
// In vb_validate schema domain
pub struct VersionTag(str);
pub struct StepId(str);
pub struct FieldName(&'static str);
pub struct TriggerKind(&'static str);
pub struct StepPrimitiveKind(&'static str);
```

### Consistency Fixes
- Standardize `"step id"` → `"step_id"` in all error messages
- Remove `"steps".to_owned()` from `MissingRequiredField` construction

---

## 6. EVIDENCE COMMANDS

To verify line count after refactor:
```bash
find crates/vb_validate/src -name "*.rs" -exec wc -l {} + | sort -n
```

To verify no single file exceeds 300:
```bash
rtk metrics crates/vb_validate/src/ --max-lines 300
```

---

## 7. VERDICT

| Check | Result |
|-------|--------|
| Line Count | 🚨 FAIL (1490 > 300) |
| Primitive Obsession | 🚨 FAIL (heavy stringly-typed) |
| DDD Cohesion | ⚠️ PARTIAL (clear clusters but no types) |
| Parse/Don't Validate | ⚠️ PARTIAL (helpers exist but raw strings) |

**ACTION REQUIRED:** Split into 6 modules + helpers. Introduce NewTypes for field names, versions, trigger kinds, and step primitives.
