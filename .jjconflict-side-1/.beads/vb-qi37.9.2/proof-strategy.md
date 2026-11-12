# Proof Strategy — vb-qi37.9.2

## Context
- **Bead**: vb-qi37.9.2
- **Title**: expr: Execute F64 bytecode semantics
- **State**: 4 (Proof Planning)
- **Isolated workspace**: /home/lewis/src/vb-qi37-9-2

## Scope Summary
F64 bytecode arithmetic and comparison in `vb_expr`:
- `eval_add_op`, `eval_sub_op`, `eval_mul_op`, `eval_div_op`, `eval_neg_op`
- `eval_gt_op`, `eval_gte_op`, `eval_lt_op`, `eval_lte_op`
- Stack bounds, I64 overflow, type mismatches
- No temporal/concurrent behavior — pure deterministic Rust computation

## Verifier Lane Selection

### TLA+
**Status**: `not_applicable`
**Rationale**: F64 bytecode eval is pure deterministic computation. No state-over-time behavior, no interleavings, no queue/scheduler/protocol/lease/retry/cancellation in scope. Confirmed by `tla-spec.md` explicit non-applicability. No TLA+ obligations planned.

### Verus
**Status**: `not_applicable`
**Rationale**: The F64 core is pure Rust with no complex refinement types, type-state predicates, or ghost/exec separation that would benefit from Verus's refinement logic. `FiniteF64::new` is a simple constructor whose correctness is fully covered by proptest (finite acceptance + NaN/Inf rejection). Verus spec gap is acknowledged in `contract.md` but no theorem kernels require it. No Verus obligations planned.

### Kani
**Status**: `planned` (proof obligations PO-001 through PO-005)
**Rationale**: F64 arithmetic with NaN/Inf handling is a bounded-state critical path. Kani can exhaustively verify that finite inputs never produce non-finite outputs through overflow, and that F64/0 produces NonFiniteFloat (not DivisionByZero). The state space (all finite f64 bit patterns) is large but Kani's bit-level bounded checking on the operation postconditions is tractable. `KANI-F64-001` covers add/sub/mul/div; `KANI-F64-002` covers F64/0 semantics. Kani harnesses do not yet exist — proof-writer must create them.

### Flux
**Status**: `not_applicable`
**Rationale**: No refinement types, dependent types, or refinement predicates in the F64 eval path. `FiniteF64` is a simple newtype with a validity predicate — not expressible as a Flux refinement in any way that adds value over proptest + Kani.

### Loom
**Status**: `not_applicable`
**Rationale**: No concurrent execution, channels, locks, atomics, async tasks, or scheduler races in the F64 bytecode eval path. Single-threaded sequential evaluation.

### Miri
**Status**: `blocked_tooling`
**Rationale**: `crates/vb_expr/src/eval.rs` and `crates/vb_core/src/value.rs` both carry `#![forbid(unsafe_code)]`. There is no unsafe code for Miri to analyze. `MIRI-001` obligation from contract phase cannot execute. This is `blocked_tooling` — not a risk gap because the code is provably safe. Compensating evidence: `CAREFUL-001` (cargo careful) provides runtime UB detection for safe Rust.

### proptest
**Status**: `planned` (proof obligations PO-006 through PO-012)
**Rationale**: F64 boundary testing (NaN, ±Inf, subnormals, signed zero, min/max finite) is best handled by structured property tests. `vb_core` already has proptest infrastructure for `FiniteF64`. `vb_expr` integration tests cover bytecode roundtrips. Proptest covers: INV-001 (finite wrapper), POST-001 through POST-006 (F64 arithmetic/comparison), INV-004 (stack bounds), ERR-003 (I64 overflow).

### fuzz
**Status**: `waived`
**Rationale**: `FUZZ-CONST-001` (deserialization boundary for ConstValue::F64) would require a `cargo fuzz` harness. No harness currently exists and creating one is outside this bead's scope. Serialization correctness is covered by existing roundtrip tests (serde roundtrip via `finite_f64_rejects_nan_returns_non_finite_number` family). Waiver issued with compensating evidence: existing test coverage.

### cargo careful
**Status**: `planned` (proof obligation PO-013)
**Rationale**: `#![forbid(unsafe_code)]` makes Miri unavailable. `cargo careful` provides runtime UB detection for safe Rust code paths. Covers the same unsafe-adjacent risk as MIRI-001 but works on safe Rust.

### static-scan (clippy + build)
**Status**: `planned` (proof obligations PO-014 and PO-015)
**Rationale**: `CLIPPY-001` and `BUILD-001` are standard machine gates. vb_expr and vb_core build cleanly in isolation.

## Obligation Summary

| Lane | Planned | Waived | Not Applicable | Blocked Tooling |
|------|---------|--------|----------------|-----------------|
| TLA+ | 0 | 0 | 5 (temporal gaps) | 0 |
| Verus | 0 | 0 | 2 (no refinement gap) | 0 |
| Kani | 2 | 0 | 0 | 0 |
| Flux | 0 | 0 | 2 (no refinement) | 0 |
| Loom | 0 | 0 | 1 (no concurrency) | 0 |
| Miri | 0 | 0 | 0 | 1 (no unsafe) |
| proptest | 7 | 0 | 0 | 0 |
| fuzz | 0 | 1 | 0 | 0 |
| cargo careful | 1 | 0 | 0 | 0 |
| clippy | 1 | 0 | 0 | 0 |
| build | 1 | 0 | 0 | 0 |
| **Total** | **13** | **1** | **11** | **1** |

## Key Assumptions
1. F64 arithmetic follows IEEE 754 natively on the target platform
2. `FiniteF64::new` is the sole gatekeeper — no f64 path bypasses it
3. Stack is bounded by `ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>` — no overflow possible beyond 64
4. No concurrent access to the eval state
5. vb_runtime build failure is DEFERRED_GLOBAL — outside scope

## Deferred / Blocked Items
- **MIRI-001**: blocked_tooling — `#![forbid(unsafe_code)]` on both crates; no unsafe code exists; `CAREFUL-001` compensates
- **FUZZ-CONST-001**: waived — no fuzz harness exists; serde roundtrip tests compensate
- **Kani harnesses**: not yet created — proof-writer must create `eval_add_op_harness`, `kani_f64_ops`, `kani_f64_div` in `crates/vb_expr/`

## Next: proof-writer (State 5)
Proof-writer must:
1. Create Kani harnesses for `eval_add_op`, `eval_sub_op`, `eval_mul_op`, `eval_div_op` verifying non-finite propagation
2. Create Kani harness for F64/0 verifying NonFiniteFloat (not DivisionByZero)
3. Confirm proptest infrastructure already exists in vb_core for FiniteF64 tests
4. Run `cargo careful test -p vb_expr` for CAREFUL-001
5. Run clippy and build for standard gates
