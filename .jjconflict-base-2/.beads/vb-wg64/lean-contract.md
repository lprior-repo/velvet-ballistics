# vb-wg64 Lean/Verus Applicability Review

## Decision

Lean and Verus proof artifacts are not applicable for this state.

## Reason

The bead scope is CI repair for formatting, lint, module resolution, and unused test warnings. The work does not introduce or modify mathematical algorithms, data-structure invariants, arithmetic contracts, parser semantics, storage recovery semantics, or runtime safety-critical state transitions.

## Contractual Proof Substitute

The required evidence is operational and review-based:

- Targeted compiler/linter commands demonstrate that known failures are repaired.
- Forced Moon CI demonstrates clean-clone acceptance.
- Diff review demonstrates the no-production-behavior-change invariant.
- Test review demonstrates assertion preservation for test-only cleanup.

## Verus Non-Applicability

No Verus specification should be added for this bead because doing so would create proof surface unrelated to the CI failure and would not bind to a meaningful changed implementation contract.

## Reconsideration Trigger

Use Verus or Lean only if later work changes executable logic with a durable mathematical invariant, such as checked arithmetic semantics, parser acceptance rules, storage recovery ordering, or runtime safety enforcement. This State 3 contract does not authorize such changes.
