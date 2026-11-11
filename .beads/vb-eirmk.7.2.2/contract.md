# vb-eirmk.7.2.2 — Reduce/Replay Error Propagation (live vs replay)

## Domain
Workflow execution and recovery via deterministic replay. Specifically: error propagation through Reduce primitives when input is wrong type or list ID is dangling.

## Goal
Verify that for Reduce workflows, both live execution and replay detect input errors and propagate typed failures. Live execution produces `RuntimeEngineError`-wrapped `EngineError` variants; replay produces `ReplayError::Internal` variants. Both fail at the same PC (the reduce step), preserve the original slot value and taint, and leave the store and IR unchanged.

## Acceptance
- **Scenario A (wrong type — I64):** Live execution of a reduce workflow reading source slot 8 produces `RuntimeEngineError::Core(TypeMismatch { expected: "list", found: "number" })`. Replay produces `ReplayError::Internal { reason: "loop slot was not list during replay" }`. Both fail at PC 1 (the reduce step). Slot 8 retains its original I64 value and DerivedFromSecret taint. Store is empty; IR is unchanged.
- **Scenario B (wrong type — Boolean):** Live execution produces `RuntimeEngineError::Core(TypeMismatch { expected: "list", found: "boolean" })`. Replay produces `ReplayError::Internal { reason: "loop slot was not list during replay" }`. Same state-preservation invariants as Scenario A.
- **Scenario C (dangling list):** Live execution produces `RuntimeEngineError::Core(ListOutOfBounds { list: ListId(u32::MAX) })`. Replay produces `ReplayError::Internal { reason: "reduce source list missing during replay" }`. Same state-preservation invariants as Scenario A.
- In all scenarios, the live error and replay error are **distinct public error enums** (`RuntimeEngineError`/`EngineError` vs `ReplayError`), intentionally different in variant structure but consistent in detecting the same logical failure.
- Two separate fresh RunFrames are used for live and replay (not reused state).
- No fabricated final PC or state; the actual PC at failure is asserted to be the reduce step.

## Non-goals
- Happy-path reduce execution
- Other primitives (collect, repeat, fold)
- Error-to-success mapping (explicitly prohibited)
- Production code edits (test-only)
- Native formal proof claims (Verus/Kani/Flux) — behavioral test coverage only

## Test Design
- Three test cases: I64 wrong-type, Boolean wrong-type, dangling ListId.
- Each test: compile reduce workflow that reads source slot 8, create two fresh RunFrames, write problematic value to slot 8 on each, drive live and replay independently, capture and match expected errors.
- Verify state preservation: PC at failure, slot value unchanged, taint unchanged, store empty, IR unchanged.
- Run: `RUSTC_WRAPPER= RUSTFLAGS=-Dwarnings cargo nextest run -p velvet-ballistics-workspace-tests --all-features --test vb_eirmk_reduce_replay_errors`

## Boundaries
- Test file only: `crates/workspace_tests/tests/vb_eirmk_reduce_replay_errors.rs`
- Contract only: `.beads/vb-eirmk.7.2.2/contract.md`
- No production edits
- No new dependencies
- No mutation of tracker/JJ state
- Pinned nightly toolchain