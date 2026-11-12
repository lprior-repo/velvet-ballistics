# Proof Writer Report: vb-5m8w Step Budget Suspension

## Scope

- State: 5 proof repair only, attempt 3.
- Workspace: `/home/lewis/src/go-skill-vb-5m8w`.
- Bead: `vb-5m8w` only.
- Production runtime behavior and normal tests were not edited. Edits were limited to TLA+ verification artifacts, `cfg(kani)` harness exposure/artifact, and bead evidence/state files.

## Artifacts Repaired

- PO-001..PO-006: `verification/tla/StepBudgetSuspension.tla` and `.cfg`.
  - `MAX_STEP_BUDGET` runnable initial state is reachable.
  - `TakeStep` from `MAX_STEP_BUDGET` decrements to `MAX_STEP_BUDGET - 1`.
  - Executable finite u64/clamp/out-of-range predicates participate in invariants/properties.
  - `MAX_U64` is exact as four executable `u16` limbs: `[65535, 65535, 65535, 65535]`; above-MAX_U64, overflow, and zero-underflow route to explicit arithmetic sink representatives.
  - Added temporal non-vacuity properties for max-budget decrement and out-of-range error routing.
- PO-008/PO-009: `crates/vb_core/src/kani_step_budget_try_take_arbitrary.rs` and `crates/vb_core/src/lib.rs` `#[cfg(kani)]` module hook.
  - Harness uses `kani::any()`/bounded assumptions for budget values and actual `RunFrame` fields.
  - It invokes production `StepBudget::try_take` zero-budget transition used by `drive_deterministic`; no immutable shadow-frame preservation claim remains.
- PO-007: Verus lane is not claimed as proof. Existing detached Verus artifacts remain waived per repaired plan.
- Reports/state: `.beads/vb-5m8w/proof-evidence.md`, `.beads/vb-5m8w/proof-writer-report.md`, `.beads/vb-5m8w/STATE.md`.

## Verification Status

- TLA+/TLC: PASS.
- Kani boundary package/lib harnesses: PASS.
- Kani structural arbitrary harness: PASS.
- Scoped nextest budget/evidence expression: PASS.
- Scoped proptest/Rust `step_budget`: PASS.
- Verus: WAIVED/TRUSTED_BOUNDARY; no Verus PASS claimed for this bead.
- Moon CI: NOT_RUN in State 5 repair; PO-012 owner_state remains State 6/global gate.

## Commands Run

```text
cargo kani --version
exit 0
cargo-kani 0.67.0
```

```text
which verus || true
exit 0
/home/lewis/.local/bin/verus
```

```text
tla2tools verification/tla/StepBudgetSuspension.tla -config verification/tla/StepBudgetSuspension.cfg
exit 0
Model checking completed. No error has been found.
6224 states generated, 3324 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 14.
```

```text
cargo kani -p vb_core --lib --harness kani_budget_sub_dim_zero --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_budget_sub_one_minus_one --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_budget_sub_one_minus_two_underflow --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_sub_dim_zero_minus_one_underflow --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_sub_dim_max_minus_max --no-assertion-reach-checks && cargo kani -p vb_core --lib --harness kani_sub_dim_max_minus_max_minus_one --no-assertion-reach-checks
exit 0
Manual Harness Summary excerpts: selected harnesses verified successfully; final summaries show 0 failures. Full raw output: /home/lewis/.local/share/opencode/tool-output/tool_e3c5940b2001e52AFVwUwFgBrG
```

```text
cargo kani -p vb_core --lib --harness kani_step_budget_try_take_arbitrary --no-assertion-reach-checks
exit 0
SUMMARY:
 ** 0 of 1939 failed
VERIFICATION:- SUCCESSFUL
Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
Verification Time: 253.06738s
Full raw output: /home/lewis/.local/share/opencode/tool-output/tool_e3c7846c8001KO1dY2O3o00Rk2
```

```text
cargo +nightly nextest run -p vb_core -p vb_runtime -E 'test(/budget|Budget|StepBudgetExhausted|AwaitingAction|AwaitingWait|AwaitingAsk|evidence/)'
exit 0
Summary [   0.063s] 426 tests run: 426 passed, 3087 skipped
```

```text
PROPTEST_CASES=1024 cargo +nightly test -p vb_core -p vb_runtime step_budget -- --nocapture
exit 0
vb_core/vb_runtime selected step_budget tests passed; notable summaries include 38 vb_core unit tests, 5 workspace budget tests, 4 adversarial/property tests, and 11 vb_runtime tests all passing.
```

## Assumptions and Bounds

- TLC representative budgets: `0..3`, `MAX_STEP_BUDGET - 3..MAX_STEP_BUDGET`, above-MAX_U64 `10001`, overflow `10002`, and zero-underflow `10003`.
- TLC exact hardware max: `MAX_U64 == [w3 |-> 65535, w2 |-> 65535, w1 |-> 65535, w0 |-> 65535]`; this is equivalent to `18446744073709551615` without TLC-unsupported large literals.
- TLC finite hardware abstraction: `U64Representable`, `OutOfRangeBudget`, `ClampMaxU64`, `ClampStepBudget`, `InvalidBudget`, and `ClampedBudgetWithinBounds` are executable operators, not comments.
- TLC PC/frame domains are finite representatives: `0..1`.
- TLC `consumed_steps` bound is `0..3`; `ModelBoundReached` closes the bounded state space after the proof budget is consumed.
- TLA+ `InjectArithmeticFault` is proof-only and enabled only for invalid/out-of-range budgets.
- Kani arbitrary harness bounds: generated actual `RunFrame` step count `1..=2`, slot count `<=2`, first step `< step_count`; budget inputs are generated `u64` and clamped by production `StepBudget::new`.
- Kani structural harness no longer uses `GeneratedRunFrameState`; it checks actual `RunFrame` observables around production zero-budget `StepBudget::try_take`.

## Remaining Blocker Packet

- No State 5 proof blocker remains for required TLA/Kani repairs.
- Verus remains intentionally waived: existing artifacts are detached/vacuum proofs unless future work binds them to actual executable Rust contracts.
- `moon ci` remains for State 6/global proof-review/CI gate.
