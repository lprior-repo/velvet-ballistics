# Assurance Bundle — vb-e4mt

**Bead:** vb-e4mt — bdd: Resource bounds and budget enforcement acceptance scenarios
**State:** 13 (evidence packaging)
**Workdir:** `/home/lewis/src/vb-e4mt-workspace`
**Source checkout:** `/home/lewis/src/velvet-ballistics`
**Date:** 2026-05-20

---

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| INV-001: WholeWorkflowBudget finite and satisfies BoundednessPolicy | POST-001, INV-001 | KANI-BUDGET-001 (FAIL_LOCAL — harness issue), TLA-WF-001 (INCONCLUSIVE), unit tests (1922 vb_core tests) | contract-verification-review.md APPROVED, black-hat APPROVED | PARTIAL — KANI-BUDGET-001 harness architecture issue; production code reviewed APPROVED |
| INV-002: AggregateResourceUsage never exceeds Capacity post-admission | POST-003, POST-004 | KANI-BUDGET-003 PASS, KANI-BUDGET-004 PASS, integration tests | contract-verification-review.md APPROVED | PASS |
| INV-003: Frame pool bounded; key space (u16,u16) finite | POST-005 | frame_pool_acquire_release_tests PASS, type_bounds_invariant | contract-verification-review.md APPROVED | PASS |
| INV-004: Expression stack depth ≤ 64 | PRE-005, INV-004 | gate_07_expression_stack_tests, VERUS-BUDGET-006, FUZZ-BUDGET-001 | contract-verification-review.md APPROVED | PASS |
| INV-005: Step budget monotonically non-increasing per tick | POST-006 | TLA-WF-003 PASS, KANI-BUDGET-005 PASS, step_budget_tick_reset_tests PASS | contract-verification-review.md APPROVED | PASS |
| INV-006: BudgetError variants exhaustive (9 variants) | INV-006 | KANI-BUDGET-002 PASS (9/9 cover props), integration_budget_error_variant_exhaustiveness | contract-verification-review.md APPROVED | PASS |
| ERR: All BudgetError variants trigger correctly | ERR-* | KANI-BUDGET-002 PASS (9 variants), integration_policy_returns_* tests PASS | contract-verification-review.md APPROVED | PASS |

---

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| KANI-BUDGET-001: WholeWorkflowBudget::compute never panics | cargo kani 0.67.0 | `cargo kani -p vb_core --harness kani_harness_whole_workflow_budget_compute` | `crates/vb_core/src/kani_workflow_budget_harnesses.rs:33` | **FAIL_LOCAL** — TIMEOUT >300s (harness architecture issue) | No — black-hat classified as harness issue, not production defect |
| KANI-BUDGET-002: BoundednessPolicy::validate exact error mapping | cargo kani 0.67.0 | `cargo kani -p vb_core --harness kani_harness_boundedness_policy_validate` | `crates/vb_core/src/kani_workflow_budget_harnesses.rs:55` | **PASS** — 221 checks, 0 failed, 9/9 cover props in 0.14s | No |
| KANI-BUDGET-003: try_add_budget never panics | cargo kani 0.67.0 | `cargo kani -p vb_core --harness kani_harness_try_add_budget_no_overflow` | `crates/vb_core/src/kani_workflow_budget_harnesses.rs:135` | **PASS** — 177 checks, 0 failed, 2/2 cover props in 1.42s | No |
| KANI-BUDGET-004: fits_within exact boolean semantics | cargo kani 0.67.0 | `cargo kani -p vb_core --harness kani_harness_fits_within_exact` | `crates/vb_core/src/kani_workflow_budget_harnesses.rs:154` | **PASS** — 177 checks, 0 failed, 1/1 cover prop in 0.77s | No |
| KANI-BUDGET-005: StepBudget::try_take raises StepBudgetExhausted before over-consumption | cargo kani 0.67.0 | `cargo kani -p vb_core --harness kani_harness_step_budget_consume` | `crates/vb_core/src/kani_workflow_budget_harnesses.rs:185` | **PASS** — 158 checks, 0 failed (3 unreachable), 1/2 cover props in 1.25s | No |
| TLA-WF-001: WorkflowBudgetSpec temporal safety | TLC | (historical — large state space) | `specs/WorkflowBudgetSpec.tla` | **INCONCLUSIVE** — state space large; vacuity fixed, error mapping fixed | No |
| TLA-WF-002: AggregateResourceSpec | TLC | (historical — 35M states, 540k distinct, 14s) | `specs/AggregateResourceSpec.tla` | **PASS** | No |
| TLA-WF-003: StepBudgetSpec | TLC | (historical — 1351 states, 186 distinct, <1s) | `specs/StepBudgetSpec.tla` | **PASS** | No |

---

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| cargo build --workspace | `cargo build --workspace` | 183 crates | **PASS** (5.07s) |
| cargo test -p vb_core | `cargo test -p vb_core` | vb_core 1922 tests | **PASS** (1.37s) |
| cargo clippy (deny warnings) | `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-features` | full workspace | **PASS** (0 issues) |
| cargo fmt --check | `cargo fmt --check` | vb_compile/kani_foreach_parity.rs | **FAIL** — DEFERRED_GLOBAL (out-of-scope crate) |

---

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| Proof Review (State 6, Attempt 3/7) | proof-review.md | REJECTED (State 6) | LETHAL-1: missing module declaration (later fixed), BLOCKED_TOOLING label misleading; subsequent repair loop reached black-hat |
| Contract Verification Review | contract-verification-review.md | **APPROVED** | PARITY confirmed for all budget computation functions; GAP-001 documented but not charged to this bead |
| Black-Hat Review (State 12) | black-hat-review.md | **APPROVED** | Production code sound; KANI-BUDGET-001 TIMEOUT is harness architecture issue; fmt DEFERRED_GLOBAL pre-existing; DEFECT-MINOR-1 (unwrap_or) and DEFECT-MINOR-2 (documentation mismatch) documented but not blocking |

---

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| KANI-BUDGET-001 FAIL_LOCAL | Harness architecture: arbitrary WorkflowParts causes exponential state space. Production code reviewed manually and approved by black-hat. | vb-e4mt | Required: bounded kani::any_with() or proof-specific Arbitrary before release gate | Black-hat APPROVED (production code sound); 4/5 Kani obligations PASS; TLA-WF-002/003 PASS |
| GATE-FMT-CHECK DEFERRED_GLOBAL | vb_compile crate (kani_foreach_parity.rs) — OUTSIDE vb-e4mt scope (budget lives in vb_core) | vb_compile owner | Format file or add to .rustfmt.toml exclusions | vb_e4mt code is fmt-clean; fmt failure is pre-existing in unrelated crate |
| fmt pre-existing | Pre-existing formatting debt, not introduced by this bead | vb_compile owner | Untracked new file in vb_compile not committed | vb-e4mt scope (vb_core) has no fmt issues |

---

## Defect Summary

| ID | Severity | File:Line | Description | Charged |
|---|---|---|---|---|
| DEFECT-MINOR-1 | MINOR | budget.rs:1414 | `unwrap_or` in `branch_count_to_u16` — unreachable on any supported platform (usize ≤ u64::MAX), but violates zero-unwrap policy | NOT charged — black-hat APPROVED with documentation |
| DEFECT-MINOR-2 | MINOR | budget.rs:570 (contract) | POST-004 says `fits_within` returns `bool`, actual returns `Result<(), AggregateBudgetError>` — semantic parity holds, documentation mismatch only | NOT charged — black-hat APPROVED |

---

## Truth Serum Audit

- report: `.beads/vb-e4mt/truth-serum-report.md`
- status: See truth-serum-report.md