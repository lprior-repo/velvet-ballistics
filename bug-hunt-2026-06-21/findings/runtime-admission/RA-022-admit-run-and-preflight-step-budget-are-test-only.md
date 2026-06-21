# RA-022: `admit_run` and `preflight_step_budget` are unused in production

- **Severity**: Info
- **Category**: simplification (dead public API)
- **Location**: `crates/vb_runtime/src/admission/admission.rs:26-54` (`admit_run`) and `crates/vb_runtime/src/admission/admission.rs:265-286` (`preflight_step_budget`)
- **Confidence**: confirmed

## Description

The two public crate-level functions `admit_run` and `preflight_step_budget` are re-exported at `admission.rs:18-22` but are only referenced by tests (`admission/tests.rs`, `admission/step_budget_tests/mod.rs`, `kani_capability_harnesses.rs`). Production code uses `admit_artifact_run` and the runtime's `preflight_step_gate` instead. The public exports invite external callers to depend on weaker admission semantics than the production path.

## Evidence

Grep over `crates/vb_runtime/src`:

- `admit_run(` — only at `admission/tests.rs:362,393,425,460,488,508` and `kani_capability_harnesses.rs:254`.
- `preflight_step_budget(` — only at `admission/step_budget_tests/mod.rs:39,53,60,67,74,87`.

`admit_run` (lines 26-54) is a strictly weaker version of `admit_artifact_run`: it checks only digest equality against the loaded artifact, NOT capability coverage, NOT verification digest binding, NOT certificate freshness. So callers using `admit_run` get a less-strict admission than the production gate.

`preflight_step_budget` (lines 265-286) duplicates `Runtime::preflight_step_gate` with weaker typing (returns `AdmissionError` instead of `RuntimeError`) and an unnecessary `AggregateResourceBudget::from_workflow` recomputation that the runtime path avoids by receiving the pre-built request.

## Adversarial Check

One could argue these are public utility APIs intended for embedders who want admission checks without the runtime facade. But the docstring on `preflight_step_budget` (`admission.rs:242-264`) explicitly describes it as "the production-extension of `admit_run_with_budget_policy`" and references "the runtime calls this together with `admit_artifact_run`" — which is false (the runtime calls `preflight_step_gate`, not `preflight_step_budget`). The docstring is misleading and the function is not actually exercised in production. Keeping it as a public API means future divergence is silent.

## Suggested Fix

Either (a) delete `admit_run` and `preflight_step_budget` and update the test callers to use the production paths; or (b) move them to a `#[cfg(any(test, feature = "test-util"))]` module so they cannot leak into production depdency graphs, and update the `preflight_step_budget` docstring to remove the false claim about runtime usage.
