# Codebase Map: vb-iucs

## Recovered Target

The rejected proof integration target is the scoped proof repair represented by `.beads/vb-qi37.8` and current source files on `main`.

## Source Files

| File | Role |
|------|------|
| `crates/vb_validate/src/kani_gate_08_accessor.rs` | Kani Gate 8 executable proof harnesses, including restored success cases. |
| `crates/vb_validate/src/gates/gate_08.rs` | Gate 8 production validation implementation. |
| `crates/vb_validate/src/shared.rs` | Shared validation pipeline entry that invokes Gate 8. |
| `crates/vb_core/src/frame.rs` | Runtime StepState transition predicate delegating to `vb_proof_kernels`. |
| `crates/vb_core/src/kani_step_state_transition.rs` | Kani parity harness over all StepState pairs. |
| `crates/vb_proof_kernels/src/step_state.rs` | Shared proof-kernel transition function used by runtime. |
| `verification/verus/step_state_machine.rs` | Verus mirror and transition lemmas for StepState. |
| `specs/tla/BudgetArithmetic.tla` | Exact-width limb model of budget add/subtract overflow/underflow behavior. |
| `specs/tla/BudgetArithmetic.cfg` | TLC config for bounded BudgetArithmetic model. |

## Artifact Files

| File | Evidence |
|------|----------|
| `.beads/vb-qi37.8/proof-evidence.md` | Raw verifier command/evidence mapping. |
| `.beads/vb-qi37.8/proof-review.md` | State 6 proof-review approval. |
| `.beads/vb-qi37.8/contract-verification-review.md` | State 6 contract-verification approval. |
| `.beads/vb-qi37.8/formal-verification-report.md` | State 11 verifier report. |
| `.beads/vb-qi37.8/black-hat-review.md` | State 12 adversarial approval. |
| `.beads/vb-qi37.8/assurance-bundle.md` | State 13 assurance summary. |
| `.beads/vb-qi37.8/truth-serum-report.md` | State 13 truth-serum audit. |
| `.beads/vb-qi37.8/final-evidence-decision.md` | Scoped landing approval. |

## Risk Tags

- proof-integration
- kani
- verus
- tla-plus
- validation-gate-8
- step-state-runtime-parity
- bounded-arithmetic
- deferred-global-nonclaims
