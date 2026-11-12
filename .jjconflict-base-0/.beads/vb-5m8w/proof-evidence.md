# Proof Evidence: vb-5m8w Step Budget Suspension

## Obligation Map

- PO-001: PASS — executable bounded u64/clamp/out-of-range semantics in `StepBudgetSuspension.tla`; `MAX_U64` is modeled exactly as four executable `u16` limbs `[65535,65535,65535,65535]`, and `BudgetWithinBounds`/`NoBudgetUnderflowOrWrap` hold.
- PO-002: PASS — `ExhaustionNonTerminal` and `LegacyTerminalExhaustionForbidden` hold.
- PO-003: PASS — reachable `RunnableState` with `budget = MAX_STEP_BUDGET`; `MaxBudgetRunnableEventuallyDecrements` checks eventual decrement to `9999`; `EvidenceRequiresConsumedBudget` holds.
- PO-004: PASS — `ExhaustionPreservesRunState` holds across `ExhaustBudget`.
- PO-005: PASS — `BudgetSuspensionEventuallyReschedulable`, `FreshBudgetEventuallyProgresses`, and `NoDeadlockExceptTerminal` hold.
- PO-006: PASS — external suspensions remain disjoint and do not emit false step success.
- PO-007: WAIVED/TRUSTED_BOUNDARY — Verus artifacts are not claimed as implementation proof.
- PO-008: PASS — package/lib Kani boundary harnesses execute on `vb_core` target.
- PO-009: PASS — package/lib Kani arbitrary/generator harness invokes production `StepBudget::try_take` zero-budget transition used by `drive_deterministic` and checks actual generated `RunFrame` observable preservation; it no longer proves immutable shadow equality.
- PO-010: PASS — scoped nextest budget/evidence selection passes.
- PO-011: PASS — scoped `step_budget` proptest/Rust tests pass with `PROPTEST_CASES=1024`.
- PO-012: NOT_RUN here — global `moon ci` remains State 6/global gate.

## Raw Evidence Summary

### TLA+/TLC

```bash
tla2tools verification/tla/StepBudgetSuspension.tla -config verification/tla/StepBudgetSuspension.cfg
```

```text
TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)
Model checking completed. No error has been found.
6224 states generated, 3324 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 14.
```

### Kani Boundary Harnesses

```bash
cargo kani -p vb_core --lib --harness kani_budget_sub_dim_zero --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_budget_sub_one_minus_one --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_budget_sub_one_minus_two_underflow --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_sub_dim_zero_minus_one_underflow --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_sub_dim_max_minus_max --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_sub_dim_max_minus_max_minus_one --no-assertion-reach-checks
```

```text
exit 0
Selected harness summaries report VERIFICATION:- SUCCESSFUL and 0 failures.
Full raw output: /home/lewis/.local/share/opencode/tool-output/tool_e3c5940b2001e52AFVwUwFgBrG
```

### Kani Structural Arbitrary Harness

```bash
cargo kani -p vb_core --lib --harness kani_step_budget_try_take_arbitrary --no-assertion-reach-checks
```

```text
SUMMARY:
 ** 0 of 1939 failed

VERIFICATION:- SUCCESSFUL
Verification Time: 253.06738s

Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

### Scoped Rust/Property Tests

```bash
cargo +nightly nextest run -p vb_core -p vb_runtime -E 'test(/budget|Budget|StepBudgetExhausted|AwaitingAction|AwaitingWait|AwaitingAsk|evidence/)'
```

```text
Summary [   0.063s] 426 tests run: 426 passed, 3087 skipped
```

```bash
PROPTEST_CASES=1024 cargo +nightly test -p vb_core -p vb_runtime step_budget -- --nocapture
```

```text
vb_core and vb_runtime selected step_budget/proptest runs completed with all selected tests passing.
```

## Trusted Boundaries / Simplifications

- TLA+ abstracts concrete Rust enum payloads into finite state/signal strings.
- TLA+ abstracts frame mutation to representative `pc`/`frame` values and completed-step counters.
- TLA+ represents exact `MAX_U64` as four executable `u16` limbs because TLC rejects 32/64-bit integer literals above its supported range. Above-MAX_U64 and overflow sinks use finite numeric representatives `10001`/`10002`; zero-underflow uses `10003`.
- TLA+ `ModelBoundReached` is a model-closure action after `consumed_steps = 3`; it is not production behavior.
- Kani arbitrary structural harness bounds generated actual `RunFrame` shapes to `step_count=1..=2`, `slot_count<=2`, and `first_step < step_count` to keep CBMC tractable. It invokes production `StepBudget::try_take` with zero budget and asserts actual frame observables are preserved around that production transition.
- Verus remains waived because current files do not bind to executable `vb_core` functions with an accepted abstraction relation.
