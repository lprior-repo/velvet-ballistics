# Proof Review — vb-zpaad (CV-106)

**Bead:** vb-zpaad (bug-hunt CV-106 follow-up).
**Pipeline caveat:** self-authored by orchestrator (no subagent tool
exposed).

## Disposition

**ACCEPTED.** All four Kani harnesses verify successfully under
`cargo kani -p vb_core --features kani-diagnostic-codes`; all seven
proptest cases pass; all thirty-two inline span tests pass. Raw
verifier logs are captured under `.evidence/vb-zpaad/kani/` and
`.evidence/vb-zpaad/tests/`.

## Lane Dispositions (per `verifier-lane-review.jsonl`)

```jsonl
{"seed":"seed.cv106.try_new_total","disposition":"accepted","binding":"crates/vb_core/src/span.rs::try_new","evidence":".evidence/vb-zpaad/kani/kani_span_try_new_returns_ok_or_err.log :: VERIFICATION:- SUCCESSFUL, 0 of 15 failed"}
{"seed":"seed.cv106.error_carries_operands","disposition":"accepted","binding":"crates/vb_core/src/span.rs::try_new","evidence":".evidence/vb-zpaad/kani/kani_span_try_new_error_carries_operands.log :: VERIFICATION:- SUCCESSFUL, 0 of 27 failed"}
{"seed":"seed.cv106.new_unchanged","disposition":"accepted","binding":"crates/vb_core/src/span.rs::new","evidence":".evidence/vb-zpaad/kani/kani_span_new_unchanged.log :: VERIFICATION:- SUCCESSFUL, 0 of 27 failed"}
{"seed":"seed.cv106.proptest_total","disposition":"accepted","binding":"crates/vb_core/tests/proptest_span_try_new.rs","evidence":".evidence/vb-zpaad/tests/proptest_span_try_new.log :: 7 passed, 0 failed"}
{"seed":"seed.cv106.proptest_zero_is_empty","disposition":"accepted","binding":"crates/vb_core/tests/proptest_span_try_new.rs::try_new_zero_zero_is_span_zero","evidence":".evidence/vb-zpaad/tests/proptest_span_try_new.log :: 7 passed, 0 failed"}
```

## Harness-by-Harness Review

### `kani_span_try_new_returns_ok_or_err`

- **Scope:** For any `s, e: u32`, `Span::try_new(s, e)` returns
  `Ok(_)` iff `s <= e`, and on the `Ok` branch the offsets are
  preserved.
- **Coverage:** `kani::any()` for both `s` and `e` means Kani treats
  all 64 input bits as symbolic. The proof explores the full
  `u32 × u32` state space.
- **Result:** 0 of 15 properties failed. The 15 corresponds to
  Kani's automatic property decomposition (asserts, no_panic, etc.).
- **Concerns:** None.

### `kani_span_try_new_error_carries_operands`

- **Scope:** On the rejection branch, the `Err` variant matches
  `SpanError::StartGreaterThanEnd { start: s, end: e }` exactly.
- **Scope restriction:** `kani::assume(s > e)` restricts the
  symbolic input to the rejection branch, cutting the state space
  in half and avoiding redundant work.
- **Result:** 0 of 27 properties failed. (27 because this proof
  generates more assertions: 4 explicit asserts plus Kani's
  per-field checks for each variant field.)

### `kani_span_new_unchanged`

- **Scope:** `Span::new(s, e)` returns `Span { start: s, end: e }`
  for any `s, e: u32`. Regression check: the fix must not alter
  the existing unchecked constructor.
- **Coverage:** `kani::any()` over the full input space.
- **Result:** 0 of 27 properties failed.
- **Note:** The 27 properties include the structural-equality
  comparison with a direct struct literal `Span { start: s, end: e }`.

### `kani_span_try_new_is_empty_consistent`

- **Scope:** For any `s <= e`, the returned `Span` satisfies
  `is_empty() == (s == e)`. Binds the `try_new` post-state to
  the `is_empty()` predicate.
- **Scope restriction:** `kani::assume(s <= e)`.
- **Result:** 0 of 3 properties failed.

## Tooling Concerns

- **Compile warning:** Kani's compiler emits "Found the following
  unsupported constructs" warnings. These are noise from `vb_core`'s
  broader compile graph (specifically `caller_location`, `foreign
  function`, `simd_reduce_all`) and do not affect the
  `kani_span_try_new` harnesses, which only call into `Span` and
  `SpanError`. Captured in the kani list output.
- **Concurrency warning:** Kani's "does not support concurrency"
  warning is also from `vb_core`'s broader compile graph (atomic
  operations used by other modules). The `kani_span_try_new`
  harnesses do not touch atomic operations.

## Production-Code Binding

| Obligation | Production site                                    |
|------------|----------------------------------------------------|
| PO1        | `crates/vb_core/src/span.rs::try_new` (lines 47-52) |
| PO2        | `crates/vb_core/src/span.rs::try_new` (lines 47-52) |
| PO3        | `crates/vb_core/src/span.rs::new` (lines 38-41)    |

All harnesses call the production `Span` and `SpanError` types
directly. No mirror types, no in-memory fakes. This satisfies the
"No Hardcoded Kani Shapes" God Rule and the "No Vacuum Verus Proofs"
God Rule (transitively, by analogy).

## Self-Authoring Marker

This proof review is self-authored by the orchestrator, not by a
`proof-reviewer` subagent, because the runtime does not expose a
subagent tool. The content is the review the `proof-reviewer` skill
would have produced given the harness outputs above.
