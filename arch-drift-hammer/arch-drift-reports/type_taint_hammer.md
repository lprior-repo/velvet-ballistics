# Architectural Drift Report: `type_taint.rs`

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_validate/src/type_taint.rs`  
**Line Count:** 591 (VIOLATION: exceeds 300-line limit by 291 lines)  
**Status:** REFACTOR REQUIRED

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 591 | 300 | ❌ OVER BY 291 |

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Raw `String` for Names and Identifiers

The following fields use raw `String` where newtypes should be applied:

| Location | Field | Issue |
|----------|-------|-------|
| `InputDecl.name` (line 115) | `String` | Should be `InputName(String)` |
| `StepTypes.id` (line 201) | `String` | Should be `StepId(String)` |
| `TypedValue::Reference(String)` (line 234) | `String` | Should be `RefPath(String)` or parsed `Reference` struct |
| `WorkflowTypes.vars` (line 188) | `Vec<(String, ValueType)>` | Should be `Vec<(VarName, ValueType)>` |
| `WorkflowTypes.secrets` (line 190) | `Vec<String>` | Should be `Vec<SecretName>` |
| `Facts.inputs` (line 391) | `HashMap<String, ValueFact>` | Keys should be `InputName` |
| `Facts.vars` (line 392) | `HashMap<String, ValueFact>` | Keys should be `VarName` |
| `Facts.secrets` (line 393) | `HashMap<String, ValueFact>` | Keys should be `SecretName` |

### 2.2 Raw `usize` for Slot Indices

| Location | Field | Issue |
|----------|-------|-------|
| `TypedValue::Slot(usize)` (line 236) | `usize` | Should be `SlotIndex(usize)` with bounded range |

### 2.3 Numeric Limits Lacking Domain Types

`ResourceLimits` (lines 123-157) uses raw `usize` for all limit fields:

```rust
pub struct ResourceLimits {
    pub max_steps: usize,              // Should be MaxSteps(usize)
    pub max_slots: usize,              // Should be MaxSlots(usize)
    pub max_constants: usize,          // Should be MaxConstants(usize)
    pub max_accessors: usize,          // Should be MaxAccessors(usize)
    pub max_expressions: usize,        // Should be MaxExpressions(usize)
    pub max_expr_stack: usize,         // Should be MaxExprStackDepth(usize)
    pub max_step_budget_per_tick: usize, // Should be MaxStepBudgetPerTick(usize)
    pub max_input_bytes: usize,         // Should be MaxInputBytes(usize)
    pub max_output_bytes: usize,       // Should be MaxOutputBytes(usize)
    pub max_blob_bytes: usize,         // Should be MaxBlobBytes(usize)
    pub max_ipc_payload_bytes: usize,  // Should be MaxIpcPayloadBytes(usize)
    pub max_retry_attempts: usize,     // Should be MaxRetryAttempts(usize)
    pub max_fanout: usize,             // Should be MaxFanout(usize)
    pub max_collect_items: usize,       // Should be MaxCollectItems(usize)
    pub max_queue_depth: usize,        // Should be MaxQueueDepth(usize)
    pub max_journal_batch_bytes: usize, // Should be MaxJournalBatchBytes(usize)
}
```

---

## 3. DDD STRUCTURAL VIOLATIONS

### 3.1 Fat Interface / God Struct: `WorkflowTypes`

`WorkflowTypes` (lines 183-195) aggregates unrelated concerns:

```rust
pub struct WorkflowTypes {
    pub inputs: Vec<InputDecl>,           // Input declarations
    pub vars: Vec<(String, ValueType)>,   // Variable declarations
    pub secrets: Vec<String>,              // Secret declarations
    pub steps: Vec<StepTypes>,             // Step definitions
    pub resource_contract: ResourceLimits, // Resource bounds
}
```

**Violation:** Mixes input schema, variable schema, security boundaries, control flow, and resource budgeting into one struct.

**DDD Principle Broken:** An `Aggregate` should group related domain objects, not orthogonal concerns like type schemas, secrets, and resource limits.

### 3.2 Unstructured Reference Parsing

`TypedValue::Reference(String)` (line 234) stores raw reference strings like `$input.user` or `$var.foo` without structural validation:

```rust
fn resolve_reference(&self, reference: &str) -> ValueFact {
    let Some(body) = reference.strip_prefix('$') else {
        return ValueFact::clean(ValueType::Text);
    };
    let Some((root, tail)) = body.split_once('.') else {
        return ValueFact::clean(ValueType::Any);
    };
    // ...
}
```

**Violation:** Reference syntax is implicitly defined and parsed via string manipulation rather than being a first-class `Reference` type with validated structure.

### 3.3 `StepKind` Non-Exhaustive

`StepKind` (line 209) is marked `#[non_exhaustive]` which may leak internal variant details to consumers.

---

## 4. FUNCTION COMPLEXITY VIOLATIONS

### 4.1 `validate_resource_limits` — 88 Lines (Should Be ≤30)

Lines 260-347 implement a monolithic function with 16 sequential `check_resource_bound` / `check_declared_bound` calls.

**Violation:** Should be refactored into a iterator-based loop or separate per-limit validators.

### 4.2 Redundant HashMap Entry Handling

Functions `input_facts`, `var_facts`, and `secret_facts` (lines 426-480) all use identical `match facts.entry(...)` patterns:

```rust
match facts.entry(name.clone()) {
    std::collections::hash_map::Entry::Occupied(mut entry) => {
        entry.insert(/* ... */);
    }
    std::collections::hash_map::Entry::Vacant(entry) => {
        entry.insert(/* ... */);
    }
}
```

**Violation:** Code duplication. Should be a generic helper or use `HashMap::entry(...).or_insert(...)`.

---

## 5. RECOMMENDED REFACTORING

### 5.1 Newtype Definitions (add to new `type_taint::types` submodule)

```rust
// crates/vb_validate/src/type_taint/types.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputName(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VarName(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretName(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StepId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefPath {
    pub root: RefRoot,
    pub segments: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefRoot {
    Input,
    Var,
    Vars,
    Secrets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotIndex(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxSteps(usize);
// ... similar for all ResourceLimits fields
```

### 5.2 File Splitting Plan

| Original File | New Files | Lines |
|---------------|-----------|-------|
| `type_taint.rs` | `type_taint/types.rs` (types + newtypes) | ~120 |
| | `type_taint/resource_limits.rs` (ResourceLimits + checks) | ~100 |
| | `type_taint/fact_table.rs` (Facts + resolution) | ~100 |
| | `type_taint/step_validation.rs` (step validators) | ~80 |
| | `type_taint.rs` (reexports + lib entry) | ~90 |

### 5.3 Factored `validate_resource_limits`

```rust
pub fn validate_resource_limits(
    workflow: &WorkflowTypes,
    hard_limits: &ResourceLimits,
) -> ValidationResult<()> {
    use crate::ValidationError::{LimitExceeded, LimitRequired};

    let limits = [
        ("max_steps", workflow.steps.len(), workflow.resource_contract.max_steps, hard_limits.max_steps),
        ("max_slots", workflow.steps.len(), workflow.resource_contract.max_slots, hard_limits.max_slots),
        // ... etc
    ];

    for (name, actual, declared, hard) in limits {
        check_declared_bound(name, declared, hard)?;
        if actual > declared {
            return Err(LimitExceeded { resource: name.to_owned() });
        }
    }
    Ok(())
}
```

---

## 6. SUMMARY

| Category | Count | Severity |
|----------|-------|----------|
| Line count violations | 1 (591 vs 300) | CRITICAL |
| Primitive obsession (String) | 8+ | HIGH |
| Primitive obsession (usize) | 16+ | MEDIUM |
| Fat structs | 1 | HIGH |
| Function complexity | 1 | MEDIUM |
| Code duplication | 3 functions | LOW |

---

**ARCHITECTURAL DRIFT STATUS: VIOLATED**  
**ACTION REQUIRED: REFACTOR BEFORE MERGE**
