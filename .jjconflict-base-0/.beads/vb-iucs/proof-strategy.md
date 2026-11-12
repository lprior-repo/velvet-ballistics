# Proof Strategy: vb-iucs

## Strategy

Use recovered, scoped current-tree proof evidence instead of inventing a new proof target.

## Lanes

- Kani Gate 8: proves accessor validation behavior for bounded valid cases and invalid edge cases.
- Kani StepState: proves production runtime predicate parity against reviewed finite transition contract over all StepState pairs.
- Verus StepState: proves transition matrix lemmas and documents production binding through `vb_proof_kernels` and Kani parity.
- TLA+ BudgetArithmetic: proves bounded arithmetic model does not deadlock and transitions overflow/underflow to typed graceful outcomes.

## Non-Claims

- Full validation pipeline composition remains `DEFERRED_GLOBAL`.
- Gate 8 Verus remains `DEFERRED_GLOBAL`.
- Gate 8 Miri remains `DEFERRED_GLOBAL`.
