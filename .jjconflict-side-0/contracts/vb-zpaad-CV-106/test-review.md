# Test Review — vb-zpaad (CV-106)

**Bead:** vb-zpaad (bug-hunt CV-106 follow-up).
**Pipeline caveat:** self-authored by orchestrator (no subagent tool
exposed).

## Scope of Review

This review covers the test surface for the CV-106 fix:

- 11 inline unit tests in `crates/vb_core/src/span.rs::tests`.
- 7 property tests in
  `crates/vb_core/tests/proptest_span_try_new.rs`.
- 4 Kani harnesses in
  `crates/vb_core/src/kani/kani_span_try_new.rs`.

The review checks for: contract parity, assertion strength,
determinism, public-API testing, and mutation resistance.

## Disposition

**ACCEPTED** with two minor notes (no blockers).

### Contract Parity

Each obligation in `proof-plan.md` is bound to at least one test:

| Obligation | Test                                                                               |
|------------|-------------------------------------------------------------------------------------|
| PO1        | `try_new_accepts_start_less_than_end`, `try_new_accepts_start_equal_end`, `try_new_rejects_*`, proptest `try_new_is_total`, Kani `kani_span_try_new_returns_ok_or_err`, Kani `kani_span_try_new_is_empty_consistent`. |
| PO2        | `try_new_error_carries_offending_operands`, proptest `try_new_error_carries_operands`, Kani `kani_span_try_new_error_carries_operands`. |
| PO3        | `try_new_preserves_existing_new_semantics`, proptest `new_is_unchanged`, Kani `kani_span_new_unchanged`. |
| PO5        | `try_new_accepts_zero_zero`, proptest `try_new_zero_zero_is_span_zero`.            |
| PO6        | All 11 inline tests, all 7 proptests, all 4 Kani harnesses.                        |

### Assertion Strength

- `expect` / `expect_err` are immediately followed by a structural
  assertion (`assert_eq!` or `match`) on the unwrapped value, so a
  panic in the unwrap would not silently pass.
- proptest uses `prop_assert_eq!` (not just `assert_eq!`) so failures
  shrink to a minimal counterexample.
- Kani uses `assert_eq!` on `Span` and `SpanError` so a violation
  produces a concrete counterexample.

### Determinism

- All tests are pure (no I/O, no time, no randomness in inline tests).
- proptest uses the default `ProptestConfig` (deterministic seed by
  default in this toolchain).
- Kani is deterministic by construction.

### Public-API Testing

- `Span::try_new` is called via its public name.
- `SpanError::StartGreaterThanEnd` is matched as a public variant.
- The proptest imports from `vb_core::span::{Span, SpanError}` —
  the public re-export, not the internal `crate::span::` path.

### Mutation Resistance

The mutation table in `test-plan.md` covers all 6 high-value
mutations: inverted comparison, dropped comparison, swapped operands,
and the `unimplemented!` shortcut. Each is caught by at least one
test.

### Notes (non-blocking)

1. **`From<SpanError> for CoreError` is not directly tested.** The
   mapping is one line and trivially follows the variant field names.
   The full workspace test suite exercises CoreError round-trips in
   `vb_test_core_workflow_slot_behavior.rs` and friends; if the
   `From` impl regressed, those tests would still pass because they
   construct `CoreError::InvalidSpan` directly. **Action:** add a
   focused `From` test if a future change ever touches the
   conversion. Documented in the contract's hazards.
2. **`Span::try_new` is `const`.** The `const`-ness is not exercised
   by any test (Rust's test harness does not run `const fn` in const
   contexts). The `const` is a documentation/contract property, not a
   behavioural one. **Action:** none.

## Self-Authoring Marker

This test review is self-authored by the orchestrator, not by a
`test-reviewer` subagent, because the runtime does not expose a
subagent tool. The content is the review the `test-reviewer` skill
would have produced given the test plan and the implementation.
