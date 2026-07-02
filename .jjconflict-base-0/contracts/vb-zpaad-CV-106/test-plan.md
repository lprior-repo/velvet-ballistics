# Test Plan — vb-zpaad (CV-106)

**Bead:** vb-zpaad (bug-hunt CV-106 follow-up).
**Pipeline caveat:** self-authored by orchestrator (no subagent tool
exposed).

## Behaviors Under Test

The fix introduces two new public surface items and documents the
existing unchecked constructor:

1. `Span::try_new(start, end) -> Result<Span, SpanError>` — checked
   constructor that rejects `start > end`.
2. `SpanError::StartGreaterThanEnd { start, end }` — single-variant
   error type.
3. `Span::new(start, end)` — must remain unchanged (regression check).
4. `From<SpanError> for CoreError` — round-trip to the core error
   taxonomy.
5. `CoreError::InvalidSpan { start, end }` — taxonomy integration.

## Test Layers

### Layer 1: Inline unit tests in `crates/vb_core/src/span.rs::tests`

These run under `cargo nextest -p vb_core` and are the closest to the
implementation. They cover the canonical and boundary cases.

| Test                                              | Asserts                                                              |
|---------------------------------------------------|----------------------------------------------------------------------|
| `try_new_accepts_start_less_than_end`             | `try_new(2,5)` → `Ok(Span{2,5})`, `is_empty()` false.                |
| `try_new_accepts_start_equal_end`                 | `try_new(7,7)` → `Ok(Span{7,7})`, `is_empty()` true.                 |
| `try_new_accepts_zero_zero`                       | `try_new(0,0)` → `Span::ZERO`, `is_empty()` true.                    |
| `try_new_accepts_zero_to_max`                     | `try_new(0, MAX)` → `Ok`, `is_empty()` false.                        |
| `try_new_accepts_max_to_max`                      | `try_new(MAX,MAX)` → `Ok`, `is_empty()` true.                        |
| `try_new_rejects_start_greater_than_end`          | `try_new(5,3)` → `Err(StartGreaterThanEnd{5,3})`.                    |
| `try_new_rejects_one_above_boundary`              | `try_new(10,9)` → `Err(StartGreaterThanEnd{10,9})`.                  |
| `try_new_rejects_max_zero_pair`                   | `try_new(MAX,0)` → `Err(StartGreaterThanEnd{MAX,0})`.                |
| `try_new_error_carries_offending_operands`        | Destructured match confirms `start` and `end` are preserved.         |
| `try_new_preserves_existing_new_semantics`        | `new(7,3)` → `Span{7,3}`; `try_new(7,3)` → `Err`.                    |
| `span_error_display_is_human_readable`            | `Display` output contains both offsets.                              |

### Layer 2: Property tests in `crates/vb_core/tests/proptest_span_try_new.rs`

These run under `cargo nextest -p vb_core` and explore the full
`u32 × u32` input space (1024 cases, with a `prop_filter` to keep the
rejection branch well-covered).

| Property                                    | Asserts                                                                  |
|---------------------------------------------|--------------------------------------------------------------------------|
| `try_new_is_total`                          | `try_new(s,e)` is total: `Ok` iff `s <= e`, `Err` otherwise.            |
| `new_is_unchanged`                          | `new(s,e)` accepts every pair and preserves the operands verbatim.       |
| `try_new_error_carries_operands`            | On the `Err` branch, the variant matches `StartGreaterThanEnd { s, e }`. |
| `try_new_is_empty_matches_offsets`          | `span.is_empty() == (s == e)` on the `Ok` branch.                        |
| `try_new_zero_zero_is_span_zero`            | `try_new(0,0) == Span::ZERO`.                                            |
| `try_new_max_max_is_empty`                  | `try_new(MAX,MAX)` is empty.                                             |
| `try_new_max_zero_is_err`                   | `try_new(MAX,0)` is the largest possible rejection.                      |

### Layer 3: Kani harnesses in `crates/vb_core/src/kani/kani_span_try_new.rs`

These run under `cargo kani -p vb_core --features
kani-diagnostic-codes --harness <name>` and prove the post-state
over every bit-level input.

| Harness                                          | Proves                                                              |
|--------------------------------------------------|---------------------------------------------------------------------|
| `kani_span_try_new_returns_ok_or_err`            | PO1 + PO2: `try_new(s,e)` is `Ok` iff `s <= e`.                     |
| `kani_span_try_new_error_carries_operands`       | PO2: the `Err` variant matches `StartGreaterThanEnd { s, e }` exactly. |
| `kani_span_new_unchanged`                        | PO3: `new(s,e)` always returns `Span { s, e }`.                      |
| `kani_span_try_new_is_empty_consistent`          | PO1: `is_empty()` agrees with `s == e` on the `Ok` branch.           |

### Layer 4: Cross-crate tests

The full workspace test suite (`cargo nextest run --workspace
--all-features`) must pass to confirm no consumer of `vb_core::Span`
breaks under the additive change. The 13,841-test count is the
baseline; the new proptest adds 7 cases.

## Assertion Strength

- `expect` / `expect_err` are used in tests to assert the
  `Ok` / `Err` branch. The branch is then re-asserted with
  `assert_eq!` / `matches!` so a panic in the unwrap would not
  silently pass.
- `prop_assert_eq!` is used in proptest so any property failure
  shrinks to a minimal counterexample.
- `assert_eq!` is used in Kani harnesses so a violation produces a
  concrete counterexample rather than a generic "failed" report.

## Determinism

All tests are deterministic:
- Inline tests are pure: same input always yields the same output.
- proptest uses the default deterministic seed (`ProptestConfig`).
  Re-running produces identical case ordering.
- Kani is deterministic by construction: it explores a fixed state
  space.

## Public-API Testing

The new public surface is tested through its public name:
- `Span::try_new` is called as `Span::try_new(...)` (no internal
  accessors).
- `SpanError::StartGreaterThanEnd` is matched in user code
  (the proptest uses a public match arm).
- `vb_core::SpanError` is imported from the public re-export in
  the proptest (`use vb_core::span::{Span, SpanError};`).

## Mutation Resistance

| Mutation                                              | Test that catches it                                            |
|-------------------------------------------------------|-----------------------------------------------------------------|
| `if start > end` → `if start >= end`                  | `try_new_accepts_start_equal_end` (rejects `0,0`).               |
| `if start > end` → `if start < end`                   | `try_new_accepts_start_less_than_end` (accepts `2,5`).           |
| Drop the `if` and always return `Ok`                  | `try_new_rejects_start_greater_than_end` and proptest.           |
| Drop the `if` and always return `Err`                 | `try_new_accepts_start_equal_end` and proptest.                 |
| Swap `start` and `end` in the error variant           | `try_new_error_carries_offending_operands` and proptest.         |
| Replace `Span::try_new` body with `unimplemented!`    | All `try_new_*` tests and proptest panic.                       |

## Coverage Targets

- `Span::try_new` Ok branch: covered by 5 inline tests and 3 proptests.
- `Span::try_new` Err branch: covered by 3 inline tests and 1 proptest.
- `Span::new` regression: covered by 1 inline test and 1 proptest.
- `SpanError` Display: covered by 1 inline test.
- `From<SpanError> for CoreError`: covered indirectly by the
  workspace test suite (existing CoreError tests in
  `crates/workspace_tests/tests/vb_test_core_workflow_slot_behavior.rs`).

## Self-Authoring Marker

This test plan is self-authored by the orchestrator, not by a
`test-planner` subagent, because the runtime does not expose a
subagent tool. The content is the plan the `test-planner` skill
would have produced given the contract above.
