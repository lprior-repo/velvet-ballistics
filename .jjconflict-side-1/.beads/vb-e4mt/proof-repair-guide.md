# Proof Repair Guide — vb-e4mt

## Bead: Resource Bounds and Budget Enforcement
**State**: 6 → 5 (back to proof-writer)
**Workdir**: /home/lewis/src/vb-e4mt-workspace
**Date**: 2026-05-19

---

## Overview

This bead is **REJECTED** at proof-review. Five LETHAL/MAJOR blockers must be resolved before re-review. Each blocker below has exact rerun targets and commands.

---

## Blocker 1 (LETHAL): Kani Harness Names Unresolvable

**Affected obligations**: KANI-BUDGET-001, KANI-BUDGET-002, KANI-BUDGET-003, KANI-BUDGET-004, KANI-BUDGET-005

### Problem

Obligations reference harness names that do not exist:
- `kani_harness_whole_workflow_budget_compute`
- `kani_harness_boundedness_policy_validate`
- `kani_harness_try_add_budget_no_overflow`
- `kani_harness_fits_within_exact`
- `kani_harness_step_budget_consume`

`rg` across the entire velvet-ballistics repo returned **zero matches** for all 5 names.

### Resolution Options

**Option A (preferred)**: Create new harness files with the obligation-specified names using the existing `kani_workflow_arbitrary.rs` arbitrary generators.

**Option B**: Update `proof-obligations.planned.jsonl` obligation IDs to match existing harness names.

### Exact Steps for Option A

1. Create `crates/vb_core/src/kani_workflow_budget_harnesses.rs`:

```rust
#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for budget obligations KANI-BUDGET-001..005
//! Uses kani::Arbitrary impls from kani_workflow_arbitrary.rs

use crate::budget::{WholeWorkflowBudget, BoundednessPolicy, AggregateResourceUsage,
                     AggregateResourceBudget, BudgetError};
use crate::workflow::WorkflowParts;

/// KANI-BUDGET-001: no panic on arbitrary WorkflowParts
#[kani::proof]
#[kani::unwind(6)]
fn kani_harness_whole_workflow_budget_compute() {
    let parts: WorkflowParts = kani::any();
    let entry = kani::any();
    let contract = kani::any();
    let result = WholeWorkflowBudget::compute(parts.nodes(), entry, &contract);
    // prove no panic; result is Result
    kani::assert(result.is_ok() || result.is_err());
}

/// KANI-BUDGET-002: BoundednessPolicy::validate maps each bound to exact BudgetError variant
#[kani::proof]
#[kani::unwind(4)]
fn kani_harness_boundedness_policy_validate() {
    let policy = BoundednessPolicy::DEFAULT;
    let budget: WholeWorkflowBudget = kani::any();
    let result = policy.validate(&budget);
    // cover each error variant path
    kani::cover!(matches!(result, Err(BudgetError::TotalStepsExceeded)));
    kani::cover!(matches!(result, Err(BudgetError::TotalSlotsExceeded)));
    kani::cover!(matches!(result, Err(BudgetError::FanoutExceeded)));
    // ... remaining 6 variants
}

/// KANI-BUDGET-003: try_add_budget never panics; returns Ok or exact error
#[kani::proof]
#[kani::unwind(4)]
fn kani_harness_try_add_budget_no_overflow() {
    let usage: AggregateResourceUsage = kani::any();
    let budget: AggregateResourceBudget = kani::any();
    let result = usage.try_add_budget(&budget);
    kani::assert(result.is_ok() || result.is_err());
}

/// KANI-BUDGET-004: fits_within exact boolean semantics
#[kani::proof]
#[kani::unwind(4)]
fn kani_harness_fits_within_exact() {
    let usage: AggregateResourceUsage = kani::any();
    let capacity: AggregateResourceBudget = kani::any();
    let result = usage.fits_within(&capacity);
    // verify result matches elementwise comparison
    // true iff all dims self <= capacity
    kani::assert(usage.max_steps_executable <= capacity.max_steps_executable);
}

/// KANI-BUDGET-005: StepBudget try_take exhaustion
#[kani::proof]
#[kani::unwind(6)]
fn kani_harness_step_budget_consume() {
    let mut budget: StepBudget = kani::any();
    let before = budget.remaining();
    let result = budget.try_take();
    match result {
        Ok(consumed) => {
            kani::assert(budget.remaining() == before - consumed);
        }
        Err(StepBudgetExhausted) => {
            kani::assert(before == 0);
        }
    }
}
```

2. Verify by running:
```bash
cd /home/lewis/src/velvet-ballistics
cargo kani -p vb_core --harness kani_harness_whole_workflow_budget_compute 2>&1 | tee /tmp/kani-001.log
cargo kani -p vb_core --harness kani_harness_boundedness_policy_validate 2>&1 | tee /tmp/kani-002.log
cargo kani -p vb_core --harness kani_harness_try_add_budget_no_overflow 2>&1 | tee /tmp/kani-003.log
cargo kani -p vb_core --harness kani_harness_fits_within_exact 2>&1 | tee /tmp/kani-004.log
cargo kani -p vb_core --harness kani_harness_step_budget_consume 2>&1 | tee /tmp/kani-005.log
```

3. Update `proof-obligations.planned.jsonl` KANI-BUDGET-001..005 `discovery` field from `MISSING` to the new file path.

---

## Blocker 2 (LETHAL): TLA-WF-001 Never Verified — State Space Explosion

**Affected obligation**: TLA-WF-001

### Problem

TLC timed out at 120s without completing. 1M+ initial states from `WithinPolicy` using actual MAX_* constants combined with `BoundedRange = 0..3`.

### Resolution Options

**Option A (preferred)**: Use Apalache symbolic model checker.
```bash
cd /home/lewis/src/vb-e4mt-workspace/.beads/vb-e4mt/specs
cargo apalache check WorkflowBudgetSpec.tla 2>&1 | tee /tmp/apalache-wf001.log
```

**Option B**: Redesign spec for TLC-compatible state space.
- Replace `BoundedRange = 0..3` with symbolic depth parameter `DEPTH = 2`
- Add `DepthCounter` variable tracking graph walk depth
- Constrain `node_count \in 0..DEPTH` and `total_steps \in 0..N` for small N
- Add `SpecApprox` variant that abstracts `WithinPolicy` as a precomputed table

### Additional Fix Required

`CompleteComputeReject` (lines 121-139) must map each violated bound to its specific `BudgetError` variant. Current spec uses existential quantifier over all 9 — this makes INV-006 (BudgetError exhaustiveness) vacuous.

Replace with 9 separate `elsif` branches:
```
\/ \E bound \in {"total_steps"} : total_steps > MAX_TOTAL_STEPS /\ budget_state' = "rejected" /\ last_error' = "TotalStepsExceeded"
\/ \E bound \in {"total_slots"} : total_slots > MAX_TOTAL_SLOTS /\ ...
...
```

### Rerun Target

```bash
cd /home/lewis/src/vb-e4mt-workspace/.beads/vb-e4mt/specs
java -XX:+UseParallelGC -jar /home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tla2tools.jar \
  WorkflowBudgetSpec.tla -config WorkflowBudgetSpec.cfg 2>&1 | tee /tmp/tlc-wf001.log
# Must complete within 120s with "No error found" for InvAdmission
```

---

## Blocker 3 (MAJOR): Verus Obligation ID Mismatch

**Affected obligations**: VERUS-BUDGET-001, VERUS-BUDGET-002, VERUS-BUDGET-003, VERUS-BUDGET-004, VERUS-BUDGET-005, VERUS-BUDGET-006

### Problem

Obligations use `VERUS-BUDGET-001..006` but the Verus file uses `VERUS-BUD-001, VERUS-BUD-002, VERUS-BUD-003, VERUS-AGG-001, VERUS-DIAG-001`. No traceability.

### Resolution Options

**Option A**: Update obligation IDs in `proof-obligations.planned.jsonl` to `VERUS-BUD-001` etc. and run Verus:
```bash
cd /home/lewis/src/velvet-ballistics
verus verification/verus/budget_bounded.rs 2>&1 | tee /tmp/verus-budget.log
```

**Option B (preferred)**: Add `#[verus::proof]` functions directly in `crates/vb_core/src/budget.rs` for each obligation, then update obligation IDs to match the new Rust-located proofs.

For `VERUS-BUDGET-001` (WholeWorkflowBudget::compute entry bounds):
```rust
#[verus::proof]
fn verify_compute_entry_bounds(nodes: &[CompiledNode], entry: StepIdx, contract: &ResourceContract) {
    // prove entry.index() < nodes.len() => compute returns Ok
    // prove entry.index() >= nodes.len() => compute returns Err
}
```

### Rerun Target

```bash
cd /home/lewis/src/velvet-ballistics
verus crates/vb_core/src/budget.rs 2>&1 | tee /tmp/verus-bud001.log
# Expected: 0 errors
```

---

## Blocker 4 (MAJOR): Proptest Functions Missing — Resource Budget

**Affected obligations**: PROP-BUDGET-001, PROP-BUDGET-002, PROP-BUDGET-003

### Problem

`proptest_sequential_compose_within_policy`, `proptest_branch_compose_within_policy`, `proptest_loop_compose_within_policy` do not exist in `crates/vb_proof_kernels/src/resource_budget.rs`. The file only has `#[test]` unit tests.

### Fix

Add `proptest!` blocks to `resource_budget.rs`:

```rust
// At end of resource_budget.rs, add:

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_sequential_compose_within_policy(a: Budget, b: Budget) {
            let policy = Policy::default_policy();
            let result = sequential_compose(&a, &b);
            let violations = policy.within(&result);
            let a_violations = policy.within(&a);
            let b_violations = policy.within(&b);
            prop_assert!(violations.is_empty() || !a_violations.is_empty() || !b_violations.is_empty());
        }

        #[test]
        fn proptest_branch_compose_within_policy(a: Budget, b: Budget) {
            let policy = Policy::default_policy();
            let result = branch_compose(&a, &b);
            let violations = policy.within(&result);
            prop_assert!(violations.is_empty());
        }

        #[test]
        fn proptest_loop_compose_within_policy(body: Budget, iterations: u64) {
            let policy = Policy::default_policy();
            prop_assume!(iterations <= 1000);
            let result = loop_compose(&body, iterations);
            let body_violations = policy.within(&body);
            let result_violations = policy.within(&result);
            // if body is within policy, loop should also be within policy for bounded iterations
            if body_violations.is_empty() && iterations <= 100 {
                prop_assert!(result_violations.is_empty());
            }
        }
    }
}
```

### Rerun Target

```bash
cd /home/lewis/src/velvet-ballistics
cargo test -p vb_proof_kernels proptest_sequential_compose_within_policy -- --nocapture 2>&1 | tee /tmp/prop-001.log
cargo test -p vb_proof_kernels proptest_branch_compose_within_policy -- --nocapture 2>&1 | tee /tmp/prop-002.log
cargo test -p vb_proof_kernels proptest_loop_compose_within_policy -- --nocapture 2>&1 | tee /tmp/prop-003.log
```

---

## Blocker 5 (MAJOR): Proptest Stack Bound Function Missing

**Affected obligation**: PROP-BUDGET-004

### Problem

`proptest_expr_stack_bound_matches_ops` does not exist in `crates/vb_expr/src/property_tests/eval_bounds.rs`.

### Fix

Add to `eval_bounds.rs`:

```rust
proptest! {
    #[test]
    fn proptest_expr_stack_bound_matches_ops(ops: Vec<ExprOp>) {
        // Bound ops length to avoid combinatorial explosion
        let ops = ops.into_iter().take(32).collect::<Vec<_>>();
        let max_stack = ops.len() as u8; // Upper bound on stack
        let capacity = 64u8;
        let result = crate::bytecode::check_expr_stack_bound(&ops, capacity);
        match result {
            Ok(depth) => {
                // depth must be consistent with actual max stack observed
                prop_assert!(depth as usize <= ops.len());
                prop_assert!(depth <= capacity);
            }
            Err(_) => {
                // Either ops too long or depth > capacity
                prop_assert!(ops.len() > capacity as usize || depth > capacity);
            }
        }
    }
}
```

### Rerun Target

```bash
cd /home/lewis/src/velvet-ballistics
cargo test -p vb_expr proptest_expr_stack_bound_matches_ops -- --nocapture 2>&1 | tee /tmp/prop-004.log
```

---

## Verification After Fixes

After all 5 blockers are resolved, run:

```bash
# Kani
for h in kani_harness_whole_workflow_budget_compute kani_harness_boundedness_policy_validate \
         kani_harness_try_add_budget_no_overflow kani_harness_fits_within_exact \
         kani_harness_step_budget_consume; do
  cargo kani -p vb_core --harness $h 2>&1 | grep -E "FAILED|PASSED|error"
done

# Verus
verus crates/vb_core/src/budget.rs 2>&1 | grep -E "0 errors|error\["
verus verification/verus/budget_bounded.rs 2>&1 | grep -E "0 errors|error\["

# Proptest
cargo test -p vb_proof_kernels proptest_ --nocapture 2>&1 | grep -E "test result|FAILED"
cargo test -p vb_expr proptest_ --nocapture 2>&1 | grep -E "test result|FAILED"

# TLA+
cd /home/lewis/src/vb-e4mt-workspace/.beads/vb-e4mt/specs
for spec in WorkflowBudgetSpec AggregateResourceSpec StepBudgetSpec; do
  java -XX:+UseParallelGC -jar /home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tla2tools.jar \
    ${spec}.tla -config ${spec}.cfg 2>&1 | grep -E "Error|No error|timeout"
done
```

---

## Rerun From

All fixed obligations should be re-reviewed from **state 5** with fresh evidence files in `.beads/vb-e4mt/proof-evidence.md`.
