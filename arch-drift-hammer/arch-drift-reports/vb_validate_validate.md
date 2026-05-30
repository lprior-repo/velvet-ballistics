# Architectural Drift Report: `vb_validate/src/ref_validate.rs`

**File Analyzed:** `crates/vb_validate/src/ref_validate.rs`  
**Lines:** 196  
**Status:** PERFECT (no refactoring required)

---

## 1. Line Count

| Metric | Value | Threshold | Pass? |
|--------|-------|-----------|-------|
| Total lines | 196 | 300 | ✅ YES |

**Verdict:** File is well within the 300-line limit.

---

## 2. DDD Cohesion Analysis

### Module Purpose
Reference validation for workflow documents. Validates that all `$input.*`, `$vars.*`, `$secrets.*`, and `$step.*` references resolve to declared names.

### Cohesion Score: HIGH

The module is **cohesive** — all types and functions serve a single, well-defined domain purpose:

| Type/Function | Responsibility |
|--------------|----------------|
| `RefTables` | Value object holding HashSets of declared names per category |
| `WorkflowRefs` | Data structure bundling inputs, vars, secrets, step_ids, and raw references |
| `validate_references()` | Entry point — builds tables and validates all references |
| `validate_single_reference()` | Core validation function — public API for cross-crate sharing |
| `string_set()` | Internal helper — converts `Vec<String>` → `HashSet<String>` |
| `reference_name()` | Internal helper — extracts name from dotted reference tail |
| `validate_bare_reference()` | Validates bare `$now`/`$random` rejects |
| `validate_rooted_reference()` | Routes validation by root (`input`, `var`, `secrets`, `step`) |
| `validate_step_reference()` | Validates step references (always rejected as runtime-time) |
| `validate_declared()` | Validates name is in declared set |

### Cross-Crate Contract (DRIFT-5)
The module explicitly exposes `RefTables` and `validate_single_reference` as public API so `vb_compile` can share validation logic without duplication. This is **intentional architectural coupling**, not drift.

---

## 3. DDD Violations

### Violation 1: Primitive Obsession — `String` for Names
**Severity:** LOW  
**Location:** `RefTables`, `WorkflowRefs`

```rust
pub struct RefTables {
    inputs: HashSet<String>,    // Should be NewType: InputNameSet
    vars: HashSet<String>,      // Should be NewType: VarNameSet
    secrets: HashSet<String>,   // Should be NewType: SecretNameSet
    step_ids: HashSet<String>,  // Should be NewType: StepIdSet
}

pub struct WorkflowRefs {
    pub inputs: Vec<String>,
    pub vars: Vec<String>,
    pub secrets: Vec<String>,
    pub step_ids: Vec<String>,
    pub references: Vec<String>,
}
```

**Impact:** Low. The validation is correct and the file is small (196 lines). Introducing NewTypes would add boilerplate without proportional safety benefit at this scale.

**Waivable:** Yes — the module is focused, the domain is narrow, and the validation is correct.

---

### Violation 2: Primitive Obsession — `&str` for References
**Severity:** LOW  
**Location:** `validate_single_reference()`, `validate_bare_reference()`, `validate_rooted_reference()`

References are passed around as raw `&str` rather than being parsed into a structured `Reference` type first (Parse, don't validate anti-pattern).

**Current pattern:**
```rust
pub fn validate_single_reference(reference: &str, tables: &RefTables) -> ValidationResult<()> {
    let Some(body) = reference.strip_prefix('$') else { return Ok(()); };
    let Some((root, tail)) = body.split_once('.') else {
        return validate_bare_reference(reference, body);
    };
    validate_rooted_reference(reference, root, tail, tables)
}
```

**Better pattern:** Parse into `Reference` enum first, then validate using the type system:
```rust
enum Reference {
    Input(InputName),
    Var(VarName),
    Secret(SecretName),
    Step(StepId),
    Runtime(RuntimeName),  // disallowed
    Unknown(String),
}
```

**Impact:** Low. Validation is correct; the module is small.

**Waivable:** Yes.

---

### Violation 3: Flat Error Algebra vs. Typed State Machine
**Severity:** LOW  
**Location:** `ValidationError` usage

The validation returns flat `ValidationError` variants (`UnknownReference`, `DirectRuntimeReference`, `FutureReference`) rather than modeling explicit state transitions. However, `ValidationError` is defined in `lib.rs` and is shared across all validation domains — it's a cross-cutting concern. Modeling reference-specific state transitions in the error type would pollute the global error enum.

**Verdict:** Acceptable. The error types are appropriately abstracted.

---

## 4. Scott Wlaschin DDD Checklist

| Rule | Status |
|------|--------|
| Make illegal states unrepresentable | ⚠️ Partial (String-based names) |
| Parse, don't validate | ⚠️ Partial (validates inline) |
| Domain types over primitives | ⚠️ Uses `String` for names |
| Explicit workflow state transitions | ✅ N/A (stateless validation) |
| Single responsibility per module | ✅ HIGH cohesion |
| No shotgun surgery | ✅ No跨 concerns |

---

## 5. Architectural Smells

| Smell | Present? | Notes |
|-------|----------|-------|
| God Object | No | Module is 196 lines, single purpose |
| Shotgun Surgery | No | All changes local to reference validation |
| Parallel Inheritance | No | Linear module structure |
| Message Chains | No | Flat function calls |
| Feature Envy | No | Data and behavior co-located |
| Introspective Coupling | No | `RefTables` exposes simple `contains_*` API |
| Middle Man | No | No unnecessary delegation |

---

## 6. Summary

| Dimension | Verdict |
|-----------|---------|
| **Line Count** | ✅ PASS (196 < 300) |
| **DDD Cohesion** | ✅ HIGH — single-purpose reference validation |
| **DDD Violations** | ⚠️ 3 minor (primitive obsession) — waivable at this scale |
| **Architectural Drift** | ✅ NONE — intentional public API for DRIFT-5 sharing |
| **Priority** | **NONE** — file is architecturally sound |

---

## Recommendation

**STATUS: PERFECT — No refactoring required.**

The file is small, cohesive, and correctly implements reference validation. The primitive obsession violations (using `String` instead of NewTypes) are acceptable trade-offs at this scale. The intentional public API for cross-crate sharing (DRIFT-5) is documented and correct.

**No beads required.**
