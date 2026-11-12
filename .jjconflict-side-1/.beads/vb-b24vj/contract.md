# vb-b24vj: Restore ask answer for wrong step behavioral test

## Problem
The prior vb-vzyac change replaced the original `shard_ask_answer_for_wrong_ask_step_returns_invalid_action_completion` behavioral test with a different test that only verifies lookup failure — it does not verify the handler's exact `RuntimeError::InvalidActionCompletion` rejection. This lost the exact behavioral contract from origin commit da633.

## Contract
A unit test `shard_ask_answer_for_wrong_ask_step_returns_invalid_action_completion` in `crates/vb_runtime/src/shard/tests/chunk_015.rs` that:

1. Submits a run that executes to an Ask node at step 2 and suspends (pending ask with timer).
2. Obtains the real issued `AskTicket` via `shard.lookup_pending_ask_ticket(run, StepIdx::new(2))`.
3. Mutates the ticket's `ask_step` to `StepIdx::new(99)` (nonexistent step) using struct update syntax on `pub(crate)` fields.
4. Enqueues `ShardCommand::AskAnswered` with the mutated ticket.
5. Asserts `shard.tick()` returns exactly `Err(RuntimeError::InvalidActionCompletion)` — the handler's real rejection path, not a lookup failure.
6. Asserts no state or journal change from the rejected answer:
   - `pending_timer_count()` remains 1 (ask not consumed).
   - `counters().snapshot().runs_completed` remains 0.
   - Journal snapshot captured before the invalid answer equals snapshot captured after.
7. Submits the original valid ticket and answer to complete the ask workflow.
8. Asserts run completes successfully: `runs_completed` becomes 1, `runs_failed` is 0, `pending_timer_count` is 0.

## Constraints
- No source changes; this is a test-only bead.
- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`.
- No derive `Deserialize`, no public constructors, no setter exposure for test convenience.
- Test must run under `nightly-2026-04-28`, edition 2024, with `-D warnings`.
- Must be run through `/tmp/vb-run-gate.py` for immutable evidence.
- The test uses `pub(crate)` field access valid from within `crates/vb_runtime`.

## Acceptance
- Test compiles and passes with exact `InvalidActionCompletion` error.
- Journal snapshot unchanged after rejected answer.
- Run completes with valid ticket after rejection.
- All 10 tests in chunk_015.rs pass.
- Full shard test suite (683 tests) passes.
