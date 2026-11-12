# Verus Review: PO-VERUS-032 Own-Patch Audit

**STATUS:** REJECTED  
**Grade claimed/reviewed:** PASS → REJECTED  
**Scope:** `verification/verus/signals_from_env.rs`, `verification/verus/extern_signals_invariant.rs`, `verification/verus/production_inner/signals_production.rs`, `contracts/proof_obligations.yaml`  
**Verifier evidence reviewed:** None (historical artifact audit; no new verification run)  
**Trust scan:** Manual scan of changed files and verifier closure via same-line text grep on source packet  
**Intactness/safe-change evidence:** Patch delta reviewed; exec body and spec contracts changed (see findings)

## Code Parity
- **Claim:** `StepBudget::from_env` returns bounded budget (remaining ≤ MAX_STEP_BUDGET) or BudgetParse error  
  **Production target:** `crates/vb_core/src/engine/signals.rs:81-94`  
  **Proof artifact:** `verification/verus/signals_from_env.rs:132-157`  
  **Gap:** Proof calls `StepBudget::new`, not `from_env`. Contract is assumed via `assume_specification`, not proven. Mirror function has fake `loop {}` body never executed in proof.

## Findings

- `CRITICAL` `verification/verus/signals_from_env.rs:132`: `assume_specification[production::step_budget_from_env]` asserts the desired result (bounded budget or BudgetParse error) without proving it. The assumption substitutes for proving the production `from_env` body. This is proof laundering per verus-code-parity.md.
- `CRITICAL` `verification/verus/signals_from_env.rs:143`: `exec_proof_from_env_composes_with_new` calls only `production::StepBudget::new(initial)`, never `production::step_budget_from_env()`. The proof demonstrates `new` clamping (already proven in PO-VERUS-006), not `from_env` boundedness. No production call bridge exists.
- `HIGH` `verification/verus/production_inner/signals_production.rs:187`: Patch changed `StepBudget::from_env` from a real method with `#[verifier::external_body]` (real body, opaque to Verus) to a standalone `step_budget_from_env` function with `#[verifier::external]` and a `loop {}` body. The function is completely disconnected from production implementation; drift check only verifies signature, not behavior.
- `HIGH` `contracts/proof_obligations.yaml:1145`: PO-VERUS-032 registry entry claims "StepBudget::from_env boundedness" is proven, but only the `StepBudget::new` clamping is proven and the `from_env` result is assumed. Claim exceeds evidence.
- `MEDIUM` `verification/verus/signals_from_env.rs:138`: `Err(_) => true` in assume_specification is weaker than production behavior. Production `from_env` only returns `Ok` or `BudgetParse` errors, not `StepCounterOverflow` or other variants. Wildcard accepts all, reducing proof informativeness.

## Trust Boundary
- `PROVED`: `StepBudget::new` clamps input to MAX_STEP_BUDGET (PO-VERUS-006, separate obligation)
- `TRUSTED`: `step_budget_from_env` returns bounded budget or BudgetParse error (assumed via `assume_specification`, not proven)
- `NEW TRUST MARKERS`: `assume_specification[production::step_budget_from_env]`, `assume_specification[production::StepBudget::new]`, `#[verifier::external]` on `step_budget_from_env`

## Verdict
The proof is rejected. It does not prove that `StepBudget::from_env` is bounded. It proves `StepBudget::new` clamps its input (already proven in PO-VERUS-006) and assumes the `from_env` result via `assume_specification`. The mirror function `step_budget_from_env` has a fake `loop {}` body that is never called in the proof, making the proof artifact disconnected from the production code. This is proof laundering: using `assume_specification` to substitute for proving production core logic. The old setup (method with `#[verifier::external_body]`) was at least a real production method whose body was opaque; the new setup is a fake function with an assumed contract.

**Potential narrow follow-up (not in scope for this audit):** A real production-bound parse helper proof without new assumptions could prove the parsing/clamping logic in isolation, but `from_env`'s env-var access would remain a trusted boundary. The current artifact should be relabeled TRUSTED, not PROVED, if retained.