# Architectural Drift Report: `admission.rs`

**File**: `crates/vb_runtime/src/admission.rs`
**Total Lines**: 1970
**Line Limit**: 300
**Overage**: 1,670 lines (557% of limit)

---

## EXECUTIVE SUMMARY

`admission.rs` is a catastrophic architectural violation. It weighs in at **1,970 lines** against a hard limit of 300 — a **557% overage**. This single file exceeds the line limit by more than 6x the entire budget.

The module has grown into a sprawling, bloated monster that violates every principle of thin-admission, DDD cohesion, and Scott Wlaschin type-driven design. It is not a policy layer — it is a **fully coupled runtime subsystem** masquerading as an admission gate.

---

## VIOLATION 1 — CATASTROPHIC LINE COUNT (1,970 >> 300)

### Anatomy of the bloat:

| Section | Lines | Purpose |
|---|---|---|
| Types + Impl | 1–890 | Core types, trait defs, store impls |
| `map_budget_error` | 795–856 | 62-line catch-all match over `AggregateBudgetError` |
| Test module `tests` | 891–1963 | 1,073 lines of inline tests |
| `include!` | 1965–1970 | Pulls in another test file via string concat |

### Verdict
**The test module alone (1,073 lines) is 3.5x the line budget.** It is inline, not in a separate test file. The `include!` at line 1969 is a code smell — tests should be proper `#[cfg(test)]` module imports, not concatenated source strings.

---

## VIOLATION 2 — ADMISSION IS NOT A THIN POLICY LAYER

Admission should be: **a decision function that accepts a policy, run context, and returns an admission result or rejection.**

What `admission.rs` actually does:

### 2.1 — Storage Backend Implementations Inside Admission

```rust
// Lines 436–497: StorageArtifactStore — STORAGE COUPLING INSIDE ADMISSION
pub struct StorageArtifactStore {
    journal: Arc<vb_storage::FjallJournal>,  // <-- Direct Fjall coupling
}
impl AcceptedArtifactStore for StorageArtifactStore { ... }
impl ArtifactStore for StorageArtifactStore { ... }
```

**Problem**: `StorageArtifactStore` is a storage implementation detail that belongs in `vb_storage`, not inside the runtime admission policy layer. The runtime should use a **`dyn ArtifactStore` trait object** — it should NEVER contain the concrete FjallJournal coupling.

This is a textbook **Dependency Inversion Principle** violation. The admission module defines the interface AND implements the concrete storage backend.

### 2.2 — Test Infrastructure Inside Admission

Lines 334–434: `AlwaysPresentArtifactStore`, `MissingAcceptedArtifactStore` — these are **test doubles**, not production admission logic.

Test doubles belong in `tests/` or behind `#[cfg(test)]` conditional compilation in a dedicated test support module, NOT embedded in the production source file.

### 2.3 — `map_budget_error` is 62 Lines of Exhaustive Matching

Lines 795–856: A 62-line match block mapping `AggregateBudgetError` variants to `AdmissionError`. This is **validation/conversion logic** that belongs in a dedicated error-mapping utility, not inline in the admission module.

---

## VIOLATION 3 — PRIMITIVE OBSESSION IN ADMISSION FUNCTIONS

### 3.1 — `u64` Primitive for Resource Dimensions

```rust
// Lines 223–229: Raw u64 for resource quantities
#[error("admission rejected: resource capacity exceeded for {resource}: {requested} > {available}")]
ResourceCapacityExceeded {
    resource: &'static str,   // <-- Primitive string for dimension name
    requested: u64,           // <-- Untyped u64
    available: u64,           // <-- Untyped u64
}
```

**Problem**: `resource: &'static str` is primitive obsession. The resource dimension should be a **typed enum** (e.g., `ResourceKind` or `BudgetDimension`), not a string. Using `&'static str` means typos in dimension names are not caught at compile time.

### 3.2 — `u64` in `AdmissionBudgetRequest`

```rust
// Lines 98–106
pub struct AdmissionBudgetRequest {
    pub requested: AggregateResourceBudget,
    pub available: AggregateResourceCapacity,
    pub policy: BoundednessPolicy,
}
```

`AggregateResourceBudget` and `AggregateResourceCapacity` are large structs with ~20 `u64` fields each. While these are not primitives in the strictest sense, the budget-checking pipeline (lines 764–773) chains 3 separate operations on these structs without a typed admission budget result:

```rust
let requested_usage = AggregateResourceUsage::default()
    .try_add_budget(&budget.requested)      // Step 1: build usage
    .map_err(map_budget_error)?;
requested_usage
    .check_policy(&budget.policy)            // Step 2: policy check
    .map_err(map_budget_error)?;
requested_usage
    .fits_within(&budget.available)         // Step 3: capacity check
    .map_err(map_budget_error)?;
```

This is a **three-step validation pipeline** that should be encapsulated in a single `AdmissionBudget::validate()` method returning a typed `AdmissionBudgetResult`, not three chained fallible operations.

### 3.3 — `ActionId` Passed as Primitive in `capability_count_mismatch_error`

```rust
// Lines 878–889
fn capability_count_mismatch_error(
    required: &[Capability],
    granted: &CapabilitySet,
) -> AdmissionError {
    let fallback = Capability::new("__capability_count_mismatch__".into(), ActionId::new(0));
    //                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Hardcoded string literal
    let required_capability = required.first().cloned().unwrap_or(fallback);
    //                                                    ^^^^^^^ unwrap_or with hardcoded fallback
```

**Problem**: `ActionId::new(0)` is a magic number. There is no `ActionId::ZERO` or typed constant for the "unknown/missing action" sentinel.

---

## VIOLATION 4 — TRAIT OBJECT TYPE ALIAS ABUSE

```rust
// Line 326
pub type SharedArtifactStore = Arc<dyn ArtifactStore>;
pub type SharedAcceptedArtifactStore = Arc<dyn AcceptedArtifactStore>;
```

These type aliases are fine, BUT the module then implements `AlwaysPresentArtifactStore`, `MissingAcceptedArtifactStore`, and `StorageArtifactStore` as concrete types in the SAME module. This violates **Module Cohesion** — the module defines traits AND implements them AND provides test doubles AND provides the production store implementation. A DDD Bounded Context should have ONE primary responsibility.

**Admission's single responsibility**: Policy decision (accept or reject).

**What admission.rs actually does**:
1. Policy decision ✓
2. Artifact envelope validation (should be in storage/artifact subsystem)
3. Storage backend implementation ✗
4. Error mapping ✗
5. Test infrastructure ✗
6. FjallJournal coupling ✗

---

## VIOLATION 5 — DUPLICATED TEST STRUCTURES (700+ lines of test boilerplate)

Lines 891–1963 contain **repeated test infrastructure**:

- `FixedAcceptedStore` struct (reimplemented 15+ times as local inline structs)
- `NeverPresentStore` (redefined 4 times)
- `AlwaysPresentStore` (redefined 2 times)
- `test_digest()`, `accepted_artifact_with_caps()` — duplicated helper functions

Example of repeated pattern across the test module:
```rust
/// Store that always returns artifacts as absent.
struct NeverPresentStore;
impl AcceptedArtifactStore for NeverPresentStore {
    fn load_accepted_artifact(&self, digest: WorkflowDigest)
        -> Result<..., ArtifactEnvelopeError> {
        Err(ArtifactEnvelopeError::ArtifactNotFound { digest })
    }
}
```

This pattern appears **at least 6 times** with minor variations.

---

## VIOLATION 6 — `include!` FOR TEST CODE

```rust
// Line 1969
include!("admission/artifact_envelope_tests.rs");
```

This is a **code smell**. Tests included via `include!` are not proper module imports — they are string-level concatenation. This makes it appear the file is 1,970 lines when the actual `admission.rs` "production" code (before tests) is ~890 lines. Even so, 890 lines is **297% of the line budget** for production code alone.

---

## RECOMMENDED REFACTORING PLAN

### Phase 1: Extract Storage Implementations (Estimated: 200 lines removed)
- Move `StorageArtifactStore` → `vb_storage/src/admission_store.rs`
- Move `AlwaysPresentArtifactStore`, `MissingAcceptedArtifactStore` → `vb_runtime/tests/support/admission_stores.rs`
- Keep only `ArtifactStore` and `AcceptedArtifactStore` trait definitions in `admission.rs`

### Phase 2: Extract Test Module (Estimated: 1,073 lines removed)
- Move the entire `#[cfg(test)] mod tests` to `vb_runtime/tests/admission_tests.rs`
- The `include!` should be replaced with proper `mod tests;` or `#[path = "..."] mod` attribute

### Phase 3: Typed Resource Dimension Enum (Estimated: 50 lines removed)
- Replace `resource: &'static str` in `AdmissionError` variants with `resource: ResourceKind`
- Define `enum ResourceKind { Steps, ActionTickets, ResultBytes, ... }`

### Phase 4: Encapsulate Budget Validation Pipeline (Estimated: 30 lines removed)
- Create `struct AdmissionBudgetCheck` that wraps the 3-step validation
- Returns `Result<AggregateResourceUsage, AdmissionError>` directly

### Phase 5: `ActionId::ZERO` Constant (Estimated: 5 lines)
- Add `ActionId::ZERO` constant in `vb_core::ids`
- Remove magic `ActionId::new(0)` in `capability_count_mismatch_error`

### Phase 6: `map_budget_error` Extraction (Estimated: 62 lines)
- Move to `vb_core/src/budget/admission_error_map.rs` or similar

---

## ARCHITECTURAL HEALTH SCORE

| Metric | Score | Status |
|---|---|---|
| Line count | 1,970 / 300 | **CRITICAL** |
| DDD Cohesion | Single module does 6 things | **CRITICAL** |
| Primitive Obsession | `&'static str`, `u64` abuse | **HIGH** |
| Storage Coupling | FjallJournal in admission | **HIGH** |
| Test Organization | Inline + include! | **HIGH** |
| Admission is Thin | No — it's a full subsystem | **CRITICAL** |

---

## SUMMARY

`admission.rs` at 1,970 lines is an architectural disaster. It is not a policy layer — it is a **runtime artifact subsystem** that happens to have "admission" in its name. The file must be split into minimum 4 modules:

1. **`admission.rs`** (policy decision only, ~200 lines): Keep `RunAdmission`, `AdmissionError`, `admit_run`, `admit_artifact_run`, `check_capability`
2. **`admission/stores.rs`** (~150 lines): Keep trait definitions only
3. **`admission/budget.rs`** (~150 lines): `AdmissionBudgetRequest`, budget validation pipeline
4. **`admission/error_map.rs`** (~70 lines): Error mapping utilities
5. **`tests/admission_tests.rs`** (~1,000 lines): All tests, with proper shared test fixtures

**TARGET: ~300 lines in `admission.rs` + ~150 in supporting modules + ~1,000 in test file = ~1,450 total, all properly separated.**

---

*Report generated by arch-drift-hammer on 2026-05-29*
*Drift violations confirmed: 6 critical, 2 high*
