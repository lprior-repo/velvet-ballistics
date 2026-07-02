# Proof Review: vb-iucs

STATUS: APPROVED

## Findings

- Target recovery is supported by `bd show vb-iucs` notes and `.beads/vb-qi37.8` artifacts.
- Gate 8 Kani evidence is executable and scoped to accessor validation behavior.
- StepState Kani binds through the production runtime predicate in `crates/vb_core/src/frame.rs`.
- Verus StepState mirror is accepted as a mirror with documented binding path, not as a direct production Verus proof.
- BudgetArithmetic TLA+ uses bounded limb arithmetic and explicit error outcomes.
- Deferred global obligations are named and not claimed as proved.

No proof repair guide required for scoped recovery.
