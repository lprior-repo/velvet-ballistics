# Regression Diff — vb-e4mt (State 11 vs State 10)

## Summary

No new verification runs were executed at State 11 — this state ran the standard machine build/test/clippy/fmt gates only. The fmt check revealed a pre-existing formatting debt in `vb_compile/src/kani_foreach_parity.rs`, an untracked file outside vb-e4mt's scope.

**No new regressions introduced by this bead's work.**

---

## Verification Status (unchanged from State 10)

| Obligation | State 10 | State 11 | Delta |
|------------|----------|----------|-------|
| KANI-BUDGET-001 (WholeWorkflowBudget::compute) | FAIL_LOCAL (timeout) | FAIL_LOCAL (timeout) | — |
| KANI-BUDGET-002 (BoundednessPolicy::validate) | PASS | PASS | — |
| KANI-BUDGET-003 (AggregateResourceUsage::try_add_budget) | PASS | PASS | — |
| KANI-BUDGET-004 (AggregateResourceUsage::fits_within) | PASS | PASS | — |
| KANI-BUDGET-005 (StepBudget::try_take) | PASS | PASS | — |
| KANI-BUDGET-ALT (word refinement) | PASS | PASS | — |
| KANI-BUDGET-ZERO (add_dim zero) | PASS | PASS | — |

---

## Machine Gate Delta

| Gate | State 10 | State 11 | Delta |
|------|----------|----------|-------|
| Build | unknown | PASS | — |
| Test | unknown | PASS (1922 tests) | — |
| Clippy | unknown | PASS | — |
| Fmt | unknown | FAIL (vb_compile fmt) | NEW FAIL (pre-existing, out-of-scope) |

---

## New Findings

### fmt failure in vb_compile (DEFERRED_GLOBAL)

**File:** `crates/vb_compile/src/kani_foreach_parity.rs`
**Status:** Untracked file (not committed to git)
**Issue:** Formatting differences — import reordering, multi-line argument style
**Scope impact:** NONE — vb_compile is outside vb-e4mt scope (budget enforcement is vb_core)

**Required action:** Format the file or add to `.rustfmt.toml` exclusions. This is pre-existing workspace debt, not bead-local.

---

## Outstanding Debt from KANI-BUDGET-001

**Obligation:** KANI-BUDGET-001 — `WholeWorkflowBudget::compute` never panics
**Status:** FAIL_LOCAL (TIMEOUT >300s)
**Root cause:** `WorkflowParts` state space explosion — deeply nested `#[kani::arbitrary]` structures with unbounded Vec/slice fields
**Required fix:** Proof-specific `kani::Arbitrary` using `kani::any_with()` to bound node slice length and field complexity
**Regression:** No new regression — same failure as State 10

---

## Conclusion

**No bead-local regressions introduced.** The fmt failure is pre-existing debt in an out-of-scope crate. KANI-BUDGET-001 timeout is unchanged from State 10 and remains the primary open issue.
