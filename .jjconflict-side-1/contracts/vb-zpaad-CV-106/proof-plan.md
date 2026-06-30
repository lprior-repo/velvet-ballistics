# Proof Plan — vb-zpaad (CV-106)

**Bead:** vb-zpaad (bug-hunt CV-106 follow-up).
**Pipeline caveat:** self-authored by orchestrator (no subagent tool
exposed). Marked at end of document.

## Scope

The bug is one missing branch in `Span::new`'s sibling constructor.
There is no async, no I/O, no allocation, no scheduler, no state
machine, and no arithmetic overflow path. The proof obligations are
therefore narrow and bounded.

## Proof Obligations

| ID  | Lane     | Claim                                                                                                       | Binding                                       |
|-----|----------|-------------------------------------------------------------------------------------------------------------|-----------------------------------------------|
| PO1 | Kani     | `Span::try_new(s, e)` returns `Ok(_)` iff `s <= e` for any `s, e: u32` in the verified range.               | `crates/vb_core/src/span.rs::try_new`         |
| PO2 | Kani     | On the `Err` path, the returned error carries the exact `start` and `end` operands.                          | `crates/vb_core/src/span.rs::try_new`         |
| PO3 | Kani     | `Span::new(s, e)` is unchanged: always returns `Span { start: s, end: e }`, including inverted inputs.        | `crates/vb_core/src/span.rs::new`             |
| PO4 | proptest | For all `s, e: u32` in proptest's default strategy, `try_new` is total (returns either `Ok` or the typed error, never panics) and `Span::new` is total. | `crates/vb_core/tests/proptest_span_try_new.rs` |
| PO5 | proptest | The `is_empty()` predicate on the result of `try_new(0, 0)` is `true`.                                       | `crates/vb_core/src/span.rs`                  |
| PO6 | inline   | Behaviour tests for the canonical cases (start < end, start == end, start > end, boundary `u32::MAX`, etc.).  | `crates/vb_core/src/span.rs::tests`           |

## Lane Decisions (per `verifier-lane-decisions.jsonl` schema)

```jsonl
{"seed":"seed.cv106.try_new_total","lane":"kani","reason":"branchless correctness over u32 x u32 is finite; kani proves both branches with explicit `kani::any()`"}
{"seed":"seed.cv106.error_carries_operands","lane":"kani","reason":"the Err variant must round-trip start and end; kani introspects the returned error"}
{"seed":"seed.cv106.new_unchanged","lane":"kani","reason":"Span::new is a no-op constructor; kani confirms we did not introduce a regression"}
{"seed":"seed.cv106.proptest_total","lane":"proptest","reason":"proptest explores u32 pairs that kani may not cover in the harness loop; both functions must be total"}
{"seed":"seed.cv106.proptest_zero_is_empty","lane":"proptest","reason":"zero-span round-trip is the only edge where the predicate and the constructor intersect"}
```

## Lane Dispositions (per `verifier-lane-review.jsonl`)

Pending until proof-reviewer passes. Will be filled in
`proof-review.md` after the Kani harness and proptest run cleanly
under the worktree's nightly toolchain.

## Tooling Runbook

- **Kani** is gated behind the `kani-diagnostic-codes` feature on
  `vb_core` (per AGENTS.md: "Bulky or stale harness groups must be
  behind package features, for example `vb_core/kani-diagnostic-codes`").
  The harness lives in
  `crates/vb_core/src/kani/kani_span_try_new.rs` and is wired up via
  `lib.rs` behind `#[cfg(all(kani, feature = "kani-diagnostic-codes"))]`.
- **proptest** lives in `crates/vb_core/tests/proptest_span_try_new.rs`
  and is part of the default test target, so it runs under
  `moon run :test` (i.e. `cargo nextest run`).
- **inline unit tests** are added to the existing `mod tests` block
  in `crates/vb_core/src/span.rs`. They run under `cargo nextest` and
  `cargo test --doc` is unaffected.

## Why No Verus / Flux / Loom / Fuzz / Miri

- **Verus:** The new function is `const fn` with one comparison and a
  struct construction. There are no ghost state, no loops, no
  pre/post conditions on heap state. A Verus spec would be
  `requires(true) && ensures(result.is_ok() == (s <= e))` — already
  what the function's body asserts by construction. Adding a Verus
  spec for a four-line function adds ceremony without coverage.
- **Flux:** Refinement types are useful for tracking resource
  budgets. A `Span` has no resource budget.
- **Loom:** No concurrent code, no threads, no shared state.
- **Fuzz:** proptest covers the `u32 × u32` input space; a libFuzzer
  harness would only widen coverage proptest already achieves.
- **Miri:** No `unsafe`, no raw pointers, no integer-pointer casts.
  The `moon run :miri` task is a smoke check on `vb_core::ids` and
  does not need to widen for this bead.

## Acceptance Criteria

1. `bash scripts/kani-list.sh vb_core` lists the new harness file.
2. `cargo kani -p vb_core --features kani-diagnostic-codes --harness
   kani_span_try_new_returns_ok_or_err` exits 0 and produces a proof
   artifact.
3. `cargo kani -p vb_core --features kani-diagnostic-codes --harness
   kani_span_try_new_error_carries_operands` exits 0.
4. `cargo kani -p vb_core --features kani-diagnostic-codes --harness
   kani_span_new_unchanged` exits 0.
5. `cargo nextest run -p vb_core --no-fail-fast` runs and passes the
   new proptest (`proptest_span_try_new`) and the inline tests.

## Self-Authoring Marker

This proof plan is self-authored by the orchestrator, not by a
`proof-planner` subagent, because the runtime does not expose a
subagent tool. The content is the plan the `proof-planner` skill
would have produced given the contract above and the existing
`vb_core` Kani harness pattern.
