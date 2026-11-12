# test-plan-review.md

bead_id: vb-core-lower-control-primitives
bead_title: "compiler: Lower v1 control primitives from YAML AST"
phase: 9
updated_at: 2026-05-15T00:00:00Z
attempt: 1

## Review Summary

Reviewed: `test-plan.md` (State 7 artifact)
Scope: Unit tests for `lower_*` functions in `vb_compile/src/lib.rs`, proptest for WaitKind exhaustiveness, clippy/build gate.

## STATUS: APPROVED

## Plan Adequacy

| Section | Assessment |
|---|---|
| Scope coverage | All 11 `lower_*` functions covered with concrete test cases |
| lower_repeat | 3 nodes + id+1 invariant verified |
| lower_ask | 2 nodes + id+1 invariant + u16::MAX overflow rejection |
| lower_choose | ChooseSlot node + empty branches rejection + branch condition tracking |
| lower_wait | Both WaitKind variants (Until, Event) + timeout field |
| SlotCompiler | slot_count and record_slot covered |
| Proptest | WaitKind exhaustiveness strategy defined |
| Clippy gate | `cargo clippy -p vb_compile` with `-D warnings` |
| Out-of-scope justification | vb-f04l blocked Kani/Miri/Verus — rationale sound |
| Execution plan | Sequential: unit tests → proptest → clippy → cargo test |

## Risk Tags

- `unit-test`: covered by direct tests
- `proptest`: WaitKind exhaustiveness via proptest strategy
- `clippy-gate`: explicit clippy invocation with `-D warnings`

## Review Notes

- Plan correctly identifies the 11 public `lower_*` functions and maps each to specific test cases.
- The `id+1` invariant for `lower_repeat` and `lower_ask` is explicitly listed as a testable requirement.
- Overflow case for `u16::MAX` is covered by `lower_ask_rejects_max_id_overflow`.
- WaitKind exhaustiveness via proptest is appropriate given only 2 variants.
- vb-f04l dependency is correctly identified as blocking Kani/Miri/Verus lanes.
- Execution plan is executable without vb-f04l landing.

## Routing

This review approves the plan. The test-writer (State 8) should execute against this plan.
Test-suite-review (State 9) will evaluate the implemented test suite against this plan.
