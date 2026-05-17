# Assurance Bundle — vb-qi37.2.5

## Bead Identity
- **Bead**: vb-qi37.2.5
- **Title**: quality: Boundedness adversarial tests
- **State**: 13 (evidence-packaging + truth-serum)
- **Scope**: Test coverage bead — no production code modified

---

## Mandatory Verification Gate Results

| Artifact | Path | Size | Status |
|----------|------|------|--------|
| delivery-scope.jsonl | .beads/vb-qi37.2.5/delivery-scope.jsonl | 4488 bytes | PRESENT |
| contract.md | .beads/vb-qi37.2.5/contract.md | 7703 bytes | PRESENT |
| traceability-matrix.jsonl | .beads/vb-qi37.2.5/traceability-matrix.jsonl | 3802 bytes | PRESENT |
| proof-review.md | .beads/vb-qi37.2.5/proof-review.md | 6111 bytes | PRESENT |
| test-plan-review.md | .beads/vb-qi37.2.5/test-plan-review.md | 1667 bytes | PRESENT |
| formal-verification-report.md | .beads/vb-qi37.2.5/formal-verification-report.md | 12273 bytes | PRESENT |
| verification-ledger.jsonl | .beads/vb-qi37.2.5/verification-ledger.jsonl | 6496 bytes | PRESENT |
| black-hat-review.md | .beads/vb-qi37.2.5/black-hat-review.md | 4743 bytes | PRESENT |
| machine-gate-report.md | .beads/vb-qi37.2.5/machine-gate-report.md | 3470 bytes | PRESENT |
| regression-diff.md | .beads/vb-qi37.2.5/regression-diff.md | — | **MISSING** |

### JSONL Validation
```
delivery-scope.jsonl: VALID
traceability-matrix.jsonl: VALID
verification-ledger.jsonl: VALID
```

### Status Approval Lines
```
formal-verification-report.md:3:STATUS: APPROVED
proof-review.md:3:STATUS: APPROVED
black-hat-review.md:3:STATUS: **APPROVED**
test-plan-review.md:45:STATUS: APPROVED
```

---

## Requirement-to-Evidence Traceability

| Contract Clause | Tests | Proofs | Review | Disposition |
|----------------|-------|--------|--------|-------------|
| INV-001 (StepBudget remaining bounded) | property_step_budget_invariant_remaining_bounded | VERUS-INV-001, KANI-INV-001 | contract-verification-review.md | COVERED |
| INV-002 (ValueStore arena cap) | property_value_store_arena_cap_enforced | VERUS-INV-002, KANI-POST-004, MIRI-INV-002 | contract-verification-review.md | COVERED (MIRI DEFERRED_GLOBAL) |
| INV-003 (count_total_steps bounded) | test_count_total_steps_respects_max_steps_per_workflow | VERUS-INV-003 | contract-verification-review.md | COVERED |
| INV-004 (run_until_blocked terminates) | test_run_until_blocked_terminates_within_budget | VERUS-INV-004, KANI-INV-004 | contract-verification-review.md | COVERED (KANI timeout compensated) |
| INV-005 (budget non-decreasing) | property_budget_non_decreasing | VERUS-INV-005 | contract-verification-review.md | COVERED |
| INV-006 (try_take monotonic) | property_try_take_monotonic_decrease | VERUS-INV-006 | contract-verification-review.md | COVERED |
| PRE-001 (StepBudget::new clamps) | property_step_budget_new_clamp, fuzz_step_budget_new_10k_runs | VERUS-INV-001, PROPTEST-PRE-001, FUZZ-001 | contract-verification-review.md | COVERED (FUZZ DEFERRED_GLOBAL) |
| PRE-002 (ValueStore cap enforced) | property_value_store_cap_enforced, test_arena_insert_returns_budget_exceeded_at_cap | VERUS-INV-002, KANI-POST-004, PROPTEST-PRE-002 | contract-verification-review.md | COVERED |
| PRE-003 (entry bounds check) | test_compute_entry_out_of_bounds_returns_error | compile_time_check | contract-verification-review.md | COVERED |
| POST-001 (try_take exact count) | property_try_take_exact_count, test_try_take_returns_true_exact_times | VERUS-INV-006, PROPTEST-POST-001, KANI-INV-001 | contract-verification-review.md | COVERED |
| POST-002 (new clamps above MAX) | property_step_budget_new_clamp, test_new_clamps_above_max | VERUS-INV-001, PROPTEST-PRE-001 | contract-verification-review.md | COVERED |
| POST-003 (StepBudgetExhausted signal) | test_run_until_blocked_returns_step_budget_exhausted, test_engine_signal_step_budget_exhausted | UNIT-POST-003, KANI-INV-004 | contract-verification-review.md | COVERED |
| POST-004 (BudgetExceeded error) | test_arena_insert_returns_budget_exceeded_at_cap, property_value_store_cap_enforced | VERUS-INV-002, KANI-POST-004, MIRI-INV-002, PROPTEST-PRE-002 | contract-verification-review.md | COVERED |
| POST-005 (StepCountOverflow error) | test_step_count_overflow_returns_error, test_compute_overflow_rejected | VERUS-INV-003, UNIT-POST-005 | contract-verification-review.md | COVERED |
| POST-006 (BoundednessPolicy validate) | test_boundedness_policy_validate_accepts_valid_budget, test_boundedness_policy_validate_rejects_over_limit, property_boundedness_policy | PROPTEST-POST-006 | contract-verification-review.md | COVERED |
| ERR-budget_exceeded | test_budget_exceeded_error_variant, test_value_store_insert_budget_exceeded | KANI-POST-004, PROPTEST-PRE-002 | contract-verification-review.md | COVERED |
| ERR-step_counter_overflow | test_step_counter_overflow_never_through_api | VERUS-INV-001, KANI-INV-001 | contract-verification-review.md | COVERED |
| ERR-entry_out_of_bounds | test_compute_entry_out_of_bounds_returns_error | compile_time_check | contract-verification-review.md | COVERED |
| ERR-nesting_depth_exceeded | test_nesting_depth_exceeded_returns_error | VERUS-INV-003 | contract-verification-review.md | COVERED |
| DEFERRED_GLOBAL-vb-runtime-chunk-001 | — | — | outside_scope | OUTSIDE_SCOPE |

---

## Execution Evidence Summary

| Metric | Claim | Evidence |
|--------|-------|----------|
| Tests Passed | 1519 | `cargo test --package vb_core --lib`: 1519 passed; 0 failed; 0 ignored |
| Line Coverage | 90.13% | nextest report (State 9), threshold ≥90% MET |
| Density Ratio | 47.5x | 1519 tests / 32 pub fns, target ≥5x MET |
| Verus Lemmas | 43 | 6 files verified, 0 errors |
| Proptest Iterations | 40,000 | 4 properties × 10,000 cases each |
| Clippy Warnings | 0 | `cargo clippy --package vb_core --lib` finished with 0 warnings |
| Production Panic Surface | 0 | rg confirms assert/unreachable only in test modules |

---

## Deferred Global Debt

| Debt | Scope | Compensating Evidence | Justification |
|------|-------|----------------------|---------------|
| FUZZ-001 | vb_runtime chunk_001.rs missing | VERUS-INV-001 (formal proof) + KANI-INV-001 (3/4 harnesses) + PROPTEST-PRE-001 (10k cases) | Pre-existing build failure, outside bead scope |
| MIRI-INV-002 | value_store timeout (300s) | VERUS-INV-002 (8 lemmas) + KANI-POST-004 (bounded model checking) + PROPTEST-PRE-002 (10k cases) | Billions of overflow allocations — legitimate gap |

---

## Waivers

| Waiver | Rationale | Compensating Evidence |
|--------|-----------|----------------------|
| TLA+ not applicable | Single-threaded deterministic loop; termination proven by VERUS-INV-004 | verification-layers.md lines 134-139 |
| Lean/Aeneas/Hax N/A | All obligations Rust-local, expressible in Verus | lean-contract.md |
| Kani loop unwind timeout | Tool limitation — exponential symbolic exploration | VERUS-INV-004 (7 lemmas) + PROPTEST-POST-001 (10k sequences) |

---

## Unresolved Items

| Item | Status | Impact |
|------|--------|--------|
| regression-diff.md | MISSING | No production code modified (black-hat noted); diff not required for test-only bead |

---

*Bundle generated: 2026-05-14*
*Evidence packaged by: femdation controller evidence-packaging state 13*
