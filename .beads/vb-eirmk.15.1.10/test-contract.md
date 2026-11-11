# Frame Pool Atomicity Behavioral Test Contract

## Scope
Add exactly four public-API integration tests in `crates/workspace_tests/tests/vb_eirmk_frame_pool_atomicity.rs` for `FramePool::take` rejection atomicity and frame recycling; do not change production code or existing tests.

## Cases
- Invalid `StepIdx(2)` in a `FramePool::new(2, 1, 1)` returns exactly `CoreError::InvalidProgramCounter { step: StepIdx(2) }`, preserves availability and capacity, and a following valid take for `RunId(2)` at `StepIdx(1)` succeeds with both identity fields correct; release restores availability to one.
- In a capacity-two pool, each invalid step in `[StepIdx(2), StepIdx::MAX, StepIdx(2)]` returns the exact typed invalid-PC error and preserves available count two and capacity two; two subsequent valid takes have distinct expected run IDs and PCs, a third returns exactly `CoreError::AllocationFailed`, and releasing both restores availability two.
- With a capacity-one pool exhausted by a held valid frame, `take` at `StepIdx::MAX` returns exactly `CoreError::AllocationFailed` (exhaustion precedence), with available zero and capacity one; after releasing the held frame, a valid take succeeds and release restores availability one.
- Mutate a taken frame through public methods (`mark_succeeded`, `write_slot_with_taint`, `increment_executed`), release it, and assert invalid `StepIdx(2)` returns the exact invalid-PC error while availability remains one; the next valid take for `RunId(9)` and `StepIdx(1)` equals an independently constructed fresh `RunFrame::new(RunId(9), StepIdx(1), 2, 1)` in full, then release restores availability one.

## Acceptance
Use exact error equality and state assertions, `Result<(), CoreError>` tests with `?` for setup, and no weakened outcomes or production changes; format only the new Rust test file with pinned nightly rustfmt and run the bead's exact scoped native nextest command, preserving its actual exit status and raw log.
