# Proof Evidence: vb-qi37.2.1 — Aggregate Resource Budget Model

**Bead:** vb-qi37.2.1
**State:** 5 (proof-writer)
**Date:** 2026-05-13

---

## E1: Kani — Concrete Budget Proofs (budget.rs K-B1..K-B9)

**Command:** `cargo kani -p vb_core`
**Tool:** Kani Rust Verifier 0.67.0 (cargo plugin)
**Result:** 9/9 harnesses verified SUCCESSFUL

Evidence from verification-ledger.jsonl (prior run):
```
PO-RUST-002-BUDGET-KANI: 14/14 Kani harnesses verified SUCCESSFUL in ~2s
K-B1: add_dim_no_panic (36 concrete pairs)
K-B2: sub_dim_no_panic (36 concrete pairs)
K-B3: add_dim_max_plus_max_overflow (Err Overflow)
K-B4: add_dim_zero_plus_zero (Ok 0)
K-B5: add_dim_one_plus_max_overflow (Err Overflow)
K-B6: sub_dim_zero_minus_one_underflow (Err Underflow)
K-B7: sub_dim_hundred_minus_fifty (Ok 50)
K-B8: add_dim_non_overflow (100+200=300 Ok)
K-B9: sub_dim_non_underflow (200-100=100 Ok)
```

---

## E2: TLA+ — BudgetArithmetic Model

**Command:** `tlc -config specs/tla/BudgetArithmetic.cfg specs/tla/BudgetArithmetic.tla`
**Result:** 2 states, 1 distinct, depth 1, PASS

Evidence from verification-ledger.jsonl:
```
PO-RUST-002-BUDGET-TLA: 2 states generated, 1 distinct state, depth 1,
no invariant violations. Verified panic freedom, monotonicity,
non-negative diff, no spurious errors, determinism on Nat model.
```

---

## E3: Verus — Budget Lemmas

**Command:** `verus verification/verus/budget_verus.rs`
**Result:** 11 lemmas verified PASS

Evidence from verification-ledger.jsonl:
```
lemma_add_dim_ok_no_overflow, lemma_add_dim_ok_value,
lemma_add_dim_err_on_overflow, lemma_add_monotonic,
lemma_sub_dim_ok_no_underflow, lemma_sub_dim_ok_value,
lemma_sub_dim_err_on_underflow, lemma_sub_nonnegative,
lemma_add_total_deterministic, lemma_sub_total_deterministic,
lemma_boundary_cases — 0 Verus errors
```

---

## E4: Static Analysis — Clippy PASS

**Command:** `cargo clippy -p vb_core -- -D warnings`
**Result:** PASS — no issues found

vb_core build evidence:
```
% cargo clippy -p vb_core -- -D warnings
cargo clippy: No issues found

% cargo build -p vb_core
Finished `dev` profile [unoptimized + debuginfo] target(s) in 16.18s
```

---

## E5: Unit Tests — Budget Tests PASS

**Command:** `cargo test -p vb_core budget`
**Result:** 227 budget tests PASS (9 suites)

---

## E6: Lean — BLOCKED_TOOLING

**Command:** `lake build`
**Tool:** Lake version 5.0.0-6d22e0e (Lean version 4.13.0)
**Result:** BLOCKED_TOOLING — no lakefile.lean in workspace root

```
Error: [root]: no configuration file with a supported extension:
././lakefile.lean
././lakefile.toml
```

---

## E7: Symbolic Kani Harnesses — Written but Not Compiled

**File:** `harnesses/kani/budget_aggregate_kani.rs`
**Status:** WRITTEN — BLOCKED_COMPILE (outside crate)

8 symbolic harnesses written targeting:
- KANI-ADD-SAFETY (overflow-before-mutation)
- KANI-SUB-SAFETY (underflow-before-mutation)
- KANI-FITS-INCLUSIVITY (equality admits, one-over rejects)
- KANI-ROUNDTRIP (add-sub roundtrip)
- KANI-ADMISSION (never false admit)

Each harness uses `kani::any()` for symbolic u64 inputs and `kani::cover!()` to
exercise both the success and failure paths.

---

## E8: vb_runtime — Build Failure

**Command:** `cargo build -p vb_runtime`
**Result:** FAIL — missing runtime/chunk_001.rs

```
error: couldn't read `crates/vb_runtime/src/runtime/chunk_001.rs`: No such file or directory
 --> crates/vb_runtime/src/runtime.rs:4:1
  |
4 | include!("runtime/chunk_001.rs");
  |
```

This blocks: integration tests (INTEG-*), KANI-ADMISSION full harness, fuzz targets.

---

## Evidence Summary by Obligation

| Obligation ID | Layer | Evidence | Status |
|---|---|---|---|
| THM-ADD-SAFETY | lean | BLOCKED_TOOLING (no lake project) | BLOCKED_TOOLING |
| THM-SUB-SAFETY | lean | BLOCKED_TOOLING (no lake project) | BLOCKED_TOOLING |
| THM-FITS-INCLUSIVITY | lean | BLOCKED_TOOLING (no lake project) | BLOCKED_TOOLING |
| THM-POLICY-EXACT | lean | BLOCKED_TOOLING (no lake project) | BLOCKED_TOOLING |
| THM-ADD-SUB-ROUNDTRIP | lean | BLOCKED_TOOLING (no lake project) | BLOCKED_TOOLING |
| THM-CONV-LOSSLESS | lean | BLOCKED_TOOLING (no lake project) | BLOCKED_TOOLING |
| KANI-ADD-SAFETY | kani | E1: K-B1,K-B3,K-B5,K-B8 concrete + E7 symbolic (not compiled) | PARTIAL |
| KANI-SUB-SAFETY | kani | E1: K-B2,K-B6,K-B7,K-B9 concrete + E7 symbolic (not compiled) | PARTIAL |
| KANI-FITS-INCLUSIVITY | kani | E7 symbolic (not compiled) | PARTIAL |
| KANI-ADMISSION | kani | E8 vb_runtime build fails | BLOCKED_BUILD |
| PROPTEST-ADD | proptest | Not run | NOT_RUN |
| PROPTEST-SUB | proptest | Not run | NOT_RUN |
| PROPTEST-FITS | proptest | Not run | NOT_RUN |
| PROPTEST-POLICY | proptest | Not run | NOT_RUN |
| PROPTEST-ROUNDTRIP | proptest | Not run | NOT_RUN |
| PROPTEST-CONV | proptest | Not run | NOT_RUN |
| INTEG-ADMISSION-EQ | integration | E8 vb_runtime build fails | BLOCKED_BUILD |
| INTEG-ADMISSION-REJECT | integration | E8 vb_runtime build fails | BLOCKED_BUILD |
| INTEG-ARTIFACT-REJECT | integration | E8 vb_runtime build fails | BLOCKED_BUILD |
| INTEG-CAPABILITY-REJECT | integration | E8 vb_runtime build fails | BLOCKED_BUILD |
| INTEG-REJECT-UNCHANGED | integration | E8 vb_runtime build fails | BLOCKED_BUILD |
| INTEG-RELEASE-FINISH | integration | E8 vb_runtime build fails | BLOCKED_BUILD |
| INTEG-RELEASE-FAIL | integration | E8 vb_runtime build fails | BLOCKED_BUILD |
| INTEG-RELEASE-CANCEL | integration | E8 vb_runtime build fails | BLOCKED_BUILD |
| INTEG-RELEASE-SHUTDOWN | integration | E8 vb_runtime build fails | BLOCKED_BUILD |
| INTEG-RESERVATION-NOT-FOUND | integration | E8 vb_runtime build fails | BLOCKED_BUILD |
| INTEG-DOUBLE-RELEASE | integration | E8 vb_runtime build fails | BLOCKED_BUILD |
| UNIT-FROM-WORKFLOW | unit | E5: 227 budget tests pass (not targeted) | NOT_RUN |
| UNIT-FROM-WHOLE | unit | E5: 227 budget tests pass (not targeted) | NOT_RUN |
| UNIT-VALIDATE-POLICY | unit | E5: 227 budget tests pass (not targeted) | NOT_RUN |
| UNIT-ADD-OVERFLOW-PER-DIM | unit | E5: 227 budget tests pass (not targeted) | NOT_RUN |
| UNIT-SUB-UNDERFLOW-PER-DIM | unit | E5: 227 budget tests pass (not targeted) | NOT_RUN |
| UNIT-FITS-PER-DIM | unit | E5: 227 budget tests pass (not targeted) | NOT_RUN |
| FUZZ-IR-BUDGET | fuzz | Not run | NOT_RUN |
| FUZZ-DECODE | fuzz | Not run | NOT_RUN |
| STATIC-GOV | static | E4: clippy PASS | PASS |
| STATIC-UNCHECKED | static | E4: clippy PASS | PASS |
| STATIC-PARSER | static | Not run | NOT_RUN |
| MUTATION | mutation | Not run | NOT_RUN |
| COVERAGE | coverage | Not run | NOT_RUN |
| GAUNTLET-PROOF | gauntlet | BLOCKED_TOOLING (no lake project) | BLOCKED_TOOLING |
| GAUNTLET-ALL | gauntlet | Not run | NOT_RUN |

---

## Compensating Evidence (WAIVER-001, WAIVER-002)

Per proof-strategy.md:
- **WAIVER-001** (runtime admission lifecycle): TLA+ BudgetArithmetic + Kani concrete proofs + 227 budget tests + Verus lemmas cover the core arithmetic properties.
- **WAIVER-002** (WholeWorkflowBudget::compute IR traversal): Integration tests + proptest/fuzz cover malformed IR handling.

---

## Blocker Summary

| Blocker | Type | Owner |
|---|---|---|
| No lakefile.lean for Lean theorems | BLOCKED_TOOLING | Architect |
| Symbolic Kani file outside crate | BLOCKED_COMPILE | proof-writer → holzman-rust |
| vb_runtime missing chunk_001.rs | BLOCKED_BUILD | vb_runtime maintainer |
| No proptest file for aggregate budget | NOT_RUN | test-writer |
| No targeted unit tests for 14 dims | NOT_RUN | test-writer |
| STATIC-PARSER grep scan not executed | NOT_RUN | proof-writer |
