# Architectural Drift Report: `kani_step_harnesses.rs`

**File**: `crates/vb_core/src/kani_step_harnesses.rs`
**Status**: REFACTOR REQUIRED
**Line Count**: 480 (VIOLATION: exceeds 300-line cap by 60%)
**Enforcer**: arch-drift-hammer
**Date**: 2026-05-29

---

## Executive Summary

This file is a **Kani proof harness library** for verifying `step_once` panic-freedom and invariants. It contains 6 proof harnesses (VB-PRE002, VB-INV002, VB-INV003, VB-INV004, VB-INV006, VB-ERR001) but suffers from:

1. **Hard violation of the <300 line rule** (480 lines, 60% over cap)
2. **Massive primitive obsession** via repeated `kani::any::<u16>()` patterns
3. **Zero abstraction reuse** — 6 identical `WorkflowParts` construction blocks
4. **Scattered bound-clamping logic** repeated 6+ times

---

## VIOLATION 1: Line Count Exceeded

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 480 | 300 | ❌ FAIL |
| Over by | 180 | 0 | ❌ FAIL |
| Utilization | 160% | 100% | ❌ FAIL |

---

## VIOLATION 2: Primitive Obsession (Scott Wlaschin DDD)

### Pattern A: Raw `u16` for bounded domain values

Every harness does this:

```rust
// ❌ PRIMITIVE OBSESSION: raw u16 for step_count
let node_count: u8 = kani::any();
kani::assume(node_count >= 1);
kani::assume(node_count <= 16);

// ❌ PRIMITIVE OBSESSION: modulo arithmetic on raw u16
let first_step_raw = kani::any::<u16>();
let first_step = StepIdx::new(first_step_raw % step_count);
```

**Problem**: `step_count` is a domain quantity with bounds [1, 16]. The modulo `% step_count` is arithmetic on a raw primitive rather than a bounded `StepCount` newtype that enforces the range at construction.

**Fix**: Introduce `BoundedStepCount<const MIN: u8, const MAX: u8>(u8)` that clamps at construction, then `first_step = StepIdx::new(kani::any::<BoundedStepCount<1, 16>>().as_step_idx())`.

### Pattern B: Identical `WorkflowParts` construction repeated 6 times

| Harness | Lines | Duplication |
|---------|-------|--------------|
| step_once_bounds_harness | 45-65 | Block A |
| step_once_state_mapping_harness | 158-170 | Block A |
| step_once_slot_init_harness | 228-240 | Block A |
| step_once_pc_bounds_harness | 289-301 | Block A |
| step_once_error_harness | 434-446 | Block A |

**Total wasted lines on duplication**: ~120 lines (6 × 20 lines).

**Fix**: Extract to `fn build_validated_workflow(kani::any()) -> Option<CompiledWorkflow>`.

### Pattern C: Repeated bound assumptions

Every harness repeats:

```rust
kani::assume(step_count >= 1);
kani::assume(step_count <= 16);
kani::assume(slot_count <= 32);
```

**Fix**: Single `fn assume_workflow_bounds(workflow: &CompiledWorkflow)` helper.

### Pattern D: `kani::any::<u16>() % effective_slot_count.max(1)` scattered

Lines 268, 362, 399 — same pattern 3 times.

**Fix**: `fn bounded_slot_idx(max: u16) -> SlotIdx`.

---

## Harness Responsibility Map

| Harness | Function | Lines | Obligation |
|---------|----------|-------|------------|
| `step_once_bounds_harness` | H1 | 39-142 | VB-PRE002: panic freedom + PRE-002 bounds |
| `step_once_state_mapping_harness` | H2 | 153-213 | VB-INV002: step-state mapping invariant |
| `step_once_slot_init_harness` | H3 | 223-275 | VB-INV003: slot initialization invariant |
| `step_once_pc_bounds_harness` | H4 | 284-333 | VB-INV004: PC bounds invariant |
| `taint_validity_harness` | H5 | 343-420 | VB-INV006: taint validity invariant |
| `step_once_error_harness` | H6 | 429-480 | VB-ERR001: error handling exhaustiveness |

---

## Required Refactors

### Refactor 1: Extract harness helpers module

Create `crates/vb_core/src/kani_harness_support.rs`:

```rust
//! Shared harness utilities — Kani-aware builders for bounded domain types.
//!
//! All functions use `kani::any()` with `kani::assume()` guards to stay
//! within proof-strategy.md bounds:
//! - step_count ∈ [1, 16]
//! - slot_count ∈ [0, 32]

use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::workflow::{CompiledWorkflow, WorkflowParts};

/// Build a validated CompiledWorkflow from arbitrary WorkflowParts.
/// Returns None if validation fails (valid PRE-002 outcome, harness skips).
pub fn arbitrary_workflow() -> Option<CompiledWorkflow> {
    let parts: WorkflowParts = kani::any();
    CompiledWorkflow::try_from_parts(parts).ok()
}

/// Return a SlotIdx within [0, max_slot).
pub fn bounded_slot_idx(max_slot: u16) -> SlotIdx {
    let raw = kani::any::<u16>() % max_slot.max(1);
    SlotIdx::new(raw)
}

/// Return a StepIdx within [0, step_count).
pub fn bounded_step_idx(step_count: u8) -> StepIdx {
    let raw = kani::any::<u16>() % step_count.max(1);
    StepIdx::new(raw)
}

/// Enforce proof-strategy.md bounds on a validated workflow.
pub fn assume_workflow_bounds(workflow: &CompiledWorkflow) {
    let step_count = workflow.node_count();
    kani::assume(step_count >= 1);
    kani::assume(step_count <= 16);
    let slot_count = workflow.slot_count();
    kani::assume(slot_count <= 32);
}
```

### Refactor 2: Shrink each harness to <80 lines

| Harness | Current | Target | Savings |
|---------|---------|--------|---------|
| step_once_bounds_harness | 104 | ~70 | ~34 |
| step_once_state_mapping_harness | 61 | ~45 | ~16 |
| step_once_slot_init_harness | 53 | ~40 | ~13 |
| step_once_pc_bounds_harness | 50 | ~35 | ~15 |
| taint_validity_harness | 78 | ~55 | ~23 |
| step_once_error_harness | 52 | ~40 | ~12 |
| **TOTAL** | **398** | **~285** | **~113** |

### Refactor 3: Create `kani_step_harnesses.rs` as a module facade

```rust
//! Kani step-proof harnesses.
//!
//! All obligations: VB-PRE002, VB-INV002, VB-INV003, VB-INV004, VB-INV006, VB-ERR001.

mod harness_support;

use crate::EngineSignal;
use crate::engine::step_once;
use crate::frame::{RunFrame, StepState};
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use crate::value_store::ValueStore;
use crate::workflow::CompiledWorkflow;

pub use harness_support::{arbitrary_workflow, assume_workflow_bounds, bounded_slot_idx, bounded_step_idx};

// H1-H6 harnesses refactored to use harness_support helpers...
```

---

## Line Count Remediation Path

| File | Current | Target | Delta |
|------|---------|--------|-------|
| `kani_step_harnesses.rs` | 480 | 300 | -180 |
| `kani_harness_support.rs` (new) | 0 | +80 | +80 |
| **Net** | 480 | 380 | -100 |

Final result: `kani_step_harnesses.rs` at 300 lines exactly (at cap, acceptable).

---

## Formal Verification

- [ ] Kani inventory updated to reflect new `kani_harness_support` module
- [ ] All 6 harness proofs still compile and pass `cargo kani`
- [ ] No behavioral change — only extraction of duplicated logic
- [ ] Evidence: `cargo kani --package vb_core --features kani-step-harnesses 2>&1 | tail -20`

---

## Severity Assessment

| Violation | Severity | Effort to Fix |
|-----------|----------|---------------|
| Line count > 300 | **CRITICAL** | Medium |
| Primitive obsession (u16 for domain values) | **HIGH** | Medium |
| 6× WorkflowParts construction duplication | **HIGH** | Low |
| Scattered bound assumptions | **MEDIUM** | Low |

**Overall**: This file is a **structural maintenance liability**. The harnesses are logically correct but architecturally丑陋. Fix the abstractions first; the proofs will remain sound.

---

## Conclusion

**STATUS**: REFACTOR REQUIRED

The file must be split into a support module + refactored harness file before it can be considered architecturally compliant. No behavior change — purely structural cleanup.
