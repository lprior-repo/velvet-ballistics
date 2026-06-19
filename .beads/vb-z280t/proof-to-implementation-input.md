# Proof → Implementation Bridge Input — vb-z280t

## Rust Source Anchors

| Claim | Source File | Lines | Function |
|---|---|---|---|
| exec_sat_loop_mul (steps field) | crates/vb_proof_kernels/src/resource_budget/budget.rs | 113 | Budget::loop_mul, field `steps` |
| exec_sat_loop_mul (actions field) | crates/vb_proof_kernels/src/resource_budget/budget.rs | 114 | Budget::loop_mul, field `actions` |
| exec_sat_loop_mul (parallel field) | crates/vb_proof_kernels/src/resource_budget/budget.rs | 115 | Budget::loop_mul, field `parallel` |
| exec_sat_loop_mul (retries field) | crates/vb_proof_kernels/src/resource_budget/budget.rs | 116 | Budget::loop_mul, field `retries` |
| exec_sat_loop_mul (gather_pages field) | crates/vb_proof_kernels/src/resource_budget/budget.rs | 117 | Budget::loop_mul, field `gather_pages` |
| exec_sat_loop_mul (gather_items field) | crates/vb_proof_kernels/src/resource_budget/budget.rs | 118 | Budget::loop_mul, field `gather_items` |
| exec_sat_loop_mul (for_each_iters field) | crates/vb_proof_kernels/src/resource_budget/budget.rs | 119 | Budget::loop_mul, field `for_each_iters` |
| exec_sat_loop_mul (together_branches field) | crates/vb_proof_kernels/src/resource_budget/budget.rs | 120 | Budget::loop_mul, field `together_branches` |
| exec_sat_loop_mul (repeat_attempts field) | crates/vb_proof_kernels/src/resource_budget/budget.rs | 121 | Budget::loop_mul, field `repeat_attempts` |
| exec_sat_loop_mul (run_time_secs field) | crates/vb_proof_kernels/src/resource_budget/budget.rs | 122 | Budget::loop_mul, field `run_time_secs` |
| exec_sat_loop_mul (result_bytes field) | crates/vb_proof_kernels/src/resource_budget/budget.rs | 123 | Budget::loop_mul, field `result_bytes` |
| exec_sat_loop_mul (slots_written field) | crates/vb_proof_kernels/src/resource_budget/budget.rs | 124 | Budget::loop_mul, field `slots_written` |

## Independent Behavior Tests

| Test | File | Type | Cases |
|---|---|---|---|
| proptest_sat_mul_u64_matches_native_mul_under_bound | crates/vb_proof_kernels/tests/proptest_resource_budget.rs | proptest | 8192 |
| proptest_sat_mul_u64_clamps_to_max_over_bound       | crates/vb_proof_kernels/tests/proptest_resource_budget.rs | proptest | 8192 |
| proptest_loop_mul_idempotent_at_zero_iterations     | crates/vb_proof_kernels/tests/proptest_resource_budget.rs | proptest | 1024 |
| proptest_loop_mul_idempotent_at_one_iteration       | crates/vb_proof_kernels/tests/proptest_resource_budget.rs | proptest | 1024 |
| proptest_loop_mul_fieldwise_saturation              | crates/vb_proof_kernels/tests/proptest_resource_budget.rs | proptest | 16384 |

## Kani Harness References

| Harness | File | Spec Function |
|---|---|---|
| kani_resource_budget_loop_mul_u64_max    | crates/vb_proof_kernels/src/verification/kani/resource_budget_loop_mul.rs | spec_sat_mul_u64 |
| kani_resource_budget_loop_mul_zero       | crates/vb_proof_kernels/src/verification/kani/resource_budget_loop_mul.rs | spec_sat_mul_u64 |
| kani_resource_budget_loop_mul_one        | crates/vb_proof_kernels/src/verification/kani/resource_budget_loop_mul.rs | spec_sat_mul_u64 |
| kani_resource_budget_loop_mul_under_u64  | crates/vb_proof_kernels/src/verification/kani/resource_budget_loop_mul.rs | spec_sat_mul_u64 |
| kani_resource_budget_loop_mul_overflow_to_max | crates/vb_proof_kernels/src/verification/kani/resource_budget_loop_mul.rs | spec_sat_mul_u64 |

## Required Evidence Commands

```
bash scripts/verify-verus.sh
bash scripts/kani-list.sh vb_proof_kernels
cargo kani --harness kani_resource_budget_loop_mul_u64_max -p vb_proof_kernels --features kani-resource-budget
cargo kani --harness kani_resource_budget_loop_mul_zero -p vb_proof_kernels --features kani-resource-budget
cargo kani --harness kani_resource_budget_loop_mul_one -p vb_proof_kernels --features kani-resource-budget
cargo kani --harness kani_resource_budget_loop_mul_under_u64 -p vb_proof_kernels --features kani-resource-budget
cargo kani --harness kani_resource_budget_loop_mul_overflow_to_max -p vb_proof_kernels --features kani-resource-budget
cargo nextest run -p vb_proof_kernels resource_budget
```

## Implementation Rule

The implementation engineer MUST NOT add `#[verifier::external_body]`,
`assume(...)`, or `axiom` to `exec_sat_loop_mul` or `exec_sat_mul_u64`.
The exec fn bodies must literally call `.saturating_mul()` to mirror
production. If the lemma cannot be proven with this restriction, fix the
spec or the production code (GOD RULE 4).