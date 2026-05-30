# Architectural Drift Report: `lifecycle_chunk_001.rs`

**File**: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`
**Total Lines**: 577
**Violation**: EXCEEDS 300-LINE LIMIT BY 277 LINES (92% OVER LIMIT)

---

## Executive Summary

This file is a monolithic god-handler for `Shard` lifecycle operations. It violates:
- **<300 Line Rule**: 577 lines (192% of allowed)
- **Single Responsibility Principle**: 7 distinct responsibility clusters
- **Primitive Obsession**: Raw tuples, slices, and primitives instead of domain types
- **DDD Cohesion**: Test helpers pollute production module

---

## Responsibility Map

| Lines | Responsibility | Concern |
|-------|----------------|---------|
| 1-58 | Submit Variants | 6 public entry points funneling to core |
| 60-136 | Submit Entry Points | Test/non-test routing for inputs/contracts |
| 138-201 | **Core Submit Orchestrator** | `handle_submit_with_inputs_contracts_and_header_mode` |
| 203-215 | Journal Event Append | Admission header persistence wrapper |
| 217-280 | Admission Building | `build_admission` — 63 lines of pure error mapping |
| 282-290 | Admission Validation | `validate_submit_admission` |
| 292-316 | Resume Handler | `handle_resume` |
| 318-368 | Resume Helpers | Validation, state checks, event append, rollback |
| 370-432 | Action Completion | `handle_action_completion` + legacy path |
| 434-506 | Action Failure | `handle_action_failure` + retry logic |
| 509-577 | Test Helpers | 69 lines of `#[cfg(test)]` utilities |

---

## Primitive Obsession Violations (DDD Bitter Truth)

### VIOLATION 1: Raw Tuple `[(SlotIdx, SlotValue)]` (Lines 65, 83, 98, 125, 142)
**Problem**: `inputs: &[(SlotIdx, SlotValue)]` is a raw slice of tuples.
**Scott Wlaschin**: "Make illegal states unrepresentable."
**Fix**: Introduce `InputSlotMap` or `SlotBindings` value object:
```rust
pub struct InputSlotMap(Vec<(SlotIdx, SlotValue)>);
```
This encapsulates iteration, lookup, and prevents empty-slice ambiguity.

### VIOLATION 2: Raw Slice `&[vb_core::action::ActionContract]` (Lines 108, 126, 144)
**Problem**: `action_contracts: &[vb_core::action::ActionContract]` passed as bare slice.
**Fix**: Wrap in `ActionContractSet`:
```rust
pub struct ActionContractSet(Box<[vb_core::action::ActionContract]>);
```
Provides `is_empty()`, `len()`, `iter()`, `contains()`.

### VIOLATION 3: Error Enum Explosion in `build_admission` (Lines 223-279)
**Problem**: 57 lines mapping `AdmissionError` → `RuntimeError` with no domain logic.
This is pure primitive transformation, not business logic.
**Fix**: Use `map_err` or a conversion trait. The `match` block is boilerplate that belongs in a `From` impl:
```rust
impl From<AdmissionError> for RuntimeError {
    fn from(e: AdmissionError) -> Self { /* ... */ }
}
```

### VIOLATION 4: Raw `retry_is_available(state, ticket, failure.retry_policy)?` (Line 498)
**Problem**: `retry_policy: VbCoreRetryPolicy` passed as primitive.
**Fix**: Encapsulate in `RetryState` or `RetryBudget` domain object on the `RunState`.

### VIOLATION 5: Untyped `Box<[vb_core::action::ActionContract]>` (Line 187)
**Problem**: `action_contracts: action_contracts.to_vec().into_boxed_slice()` — raw boxed slice.
**Fix**: Already proposed `ActionContractSet` above.

---

## Single Responsibility Violations

### CLUSTER 1: Submit Variants (Lines 1-136)
**Issue**: 6 public methods + 1 private core method = 7 methods all doing routing.
**Problem**: `#[cfg(test)]` conditional compilation splits `handle_submit` and `handle_submit_with_inputs` into two versions each.
**Fix**: Use a `SubmitContext` struct that encapsulates inputs/contracts/header_mode:
```rust
struct SubmitContext {
    inputs: InputSlotMap,
    contracts: ActionContractSet,
    persist_header: bool,
}
```
Then one `handle_submit_impl` method.

### CLUSTER 2: Test Helpers (Lines 509-577)
**Issue**: 69 lines of `#[cfg(test)]` code in a production module.
**Problem**: Pollutes module namespace, increases compile times, confuses static analysis.
**Fix**: Move to `shard/lifecycle/chunk_001_test_helpers.rs` behind `#[cfg(test)]` module boundary or separate test file.

### CLUSTER 3: Admission Error Mapping (Lines 217-280)
**Issue**: 63 lines of 1:1 error enum mapping.
**Fix**: Replace with `impl From<AdmissionError> for RuntimeError` in `admission` module.

---

## File Bloat Metrics

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 577 | 300 | 🔴 OVER |
| Functions | 14 | ~6 | 🔴 OVER |
| `#[cfg(test)]` blocks | 5 | 0 in prod | 🔴 OVER |
| Error mapping arms | 14 | 0 | 🔴 OVER |

---

## Refactoring Prescription

### Phase 1: Extract Value Objects (Do not touch production logic)
1. Create `vb_runtime/src/shard/lifecycle/inputs.rs`:
   - `InputSlotMap` — wraps `Vec<(SlotIdx, SlotValue)>`
   - `InputSlotMap::new()`, `InputSlotMap::is_empty()`, `InputSlotMap::get()`, `InputSlotMap::iter()`
2. Create `vb_runtime/src/shard/lifecycle/contracts.rs`:
   - `ActionContractSet` — wraps `Vec<vb_core::action::ActionContract>`
   - `ActionContractSet::new()`, `is_empty()`, `len()`, `iter()`, `contains()`

### Phase 2: Move Admission Error Mapping
1. Add `impl From<AdmissionError> for RuntimeError` in `admission` module
2. Replace `build_admission` match block with `.map_err(Into::into)`

### Phase 3: Collapse Submit Variants
1. Replace 7 methods with 1 core + `SubmitContext` struct
2. Use builder pattern for `SubmitContext`

### Phase 4: Extract Test Helpers
1. Create `shard/lifecycle/test_helpers.rs`
2. Import into `#[cfg(test)]` module

### Phase 5: Final Polish
- Move `append_admission_header_journal_event` into a journal helper module
- Collapse `handle_legacy_action_completion` into completion handler with flag

---

## Verdict

**ARCHITECTURAL DRIFT**: 🔴 **SEVERE**

This file is a prime example of "god module" antipattern in DDD. It:
- Exceeds line limit by 92%
- Mixes 7 responsibility clusters
- Uses raw primitives where domain types should exist
- Puts test code in production module

**Required Actions**:
1. Extract value objects (`InputSlotMap`, `ActionContractSet`)
2. Move test helpers to separate file
3. Replace error mapping with `From` impl
4. Collapse submit variants with `SubmitContext`
5. Target: ≤250 lines after refactoring

---

*Generated by arch-drift-hammer on 2026-05-29*
