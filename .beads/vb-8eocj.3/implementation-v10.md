# Implementation v10: ForEach/Reduce Owned Loop Reentry

## Status
Draft — not yet compiled or test-validated. Root will run strict source tests.

## Changed Paths
1. `crates/vb_runtime/src/primitives/for_each.rs` — `for_each_next` loop reentry fix
2. `crates/vb_runtime/src/primitives/reduce.rs` — `reduce_next` loop reentry fix
3. `crates/vb_core/src/workflow/mod.rs` — push_loop_span finished-pop-before-compare (preserved from predecessor)

## Cleanup Performed
- Removed generic `(StepState::Succeeded, StepState::Running)` transition from `crates/vb_core/src/frame.rs` (predecessor rejected change). Generic state table now equals baseline.
- Reverted `crates/vb_compile/src/mod_compile_lowering/part_04.rs` RepeatCheck conversion to exact baseline. Repeated next backwards edge remains unvalidated; Repeat deferred to separate bead.
- Removed predecessor-created `.cargo/config.local.toml`, `.cargo/config.sandbox.toml`, `.cargo/config.toml.bak`. Baseline `.cargo/config.toml` preserved.

## Problem
ForEach/Reduce loop continuation nodes (ForEachNext/ReduceNext) are re-entered multiple times (once per iteration). The generic state machine only allows each step to transition Succeeded once. After the first iteration, the loop continuation node is marked Succeeded. On the second iteration, when the drive loop returns to the loop continuation node, `begin_drive_step` tries to mark it Running, but the Succeeded->Running transition is not in the generic state table — causing failure.

Existing `jump_to_body` helper already handles the body step's state (resets it to Pending if Succeeded), but does not handle the loop continuation node itself.

## Fix
In `for_each_next` and `reduce_next`, save the current step (the loop continuation node) before calling `jump_to_body`, then reset the saved step to Pending after `jump_to_body` returns. This makes the loop continuation node re-enterable for subsequent iterations.

Invariant argument: The loop continuation node owns the iteration boundary. It is legitimate for it to reset its own state to Pending as part of setting up the next iteration, because:
1. The node is the owner of the loop's iteration logic (it controls the cursor/iterator state).
2. The reset happens at a legitimate iteration boundary (after binding the next item and before jumping to the body).
3. The generic state table is preserved — no new transitions are added.
4. Unrelated nodes are unaffected.
5. Wrong-state errors and error propagation are preserved (mark_pending uses the standard RunFrame state validation).

The fix uses existing `mark_pending` API — no new invariants or blanket allowances are introduced.

## Tests Not Run (Root Will Run)
- Source19 fixture (baseline compiles and runs 5 PASS 14 FAIL)
- ForEach two-items trace (should show StepIdx2 Succeeded->Running after second body)
- Existing ForEach/Reduce test suites
- Reduced loop iteration tests
- Crossing-loop and invalid-state negative tests
- Together completion tests (separate issue, not addressed here)

## Remaining Gaps (Not Addressed)
1. **Repeat**: Repeated next backwards edge remains unvalidated. RepeatCheck/Attempt wiring deferred to separate bead.
2. **Together**: Results and continuation use consistent initialized runtime state — separate issue.
3. **Collect**: Executes first nonempty page body even when source fits one page — separate issue.
4. **Evidence gaps**: Slot evidence emission for loop reentry not yet verified.
5. **Compile-only validation**: No cargo/rustc/moon run performed; syntax and type correctness unverified.

## Design Decisions
- Fix applied in loop continuation node handlers (`for_each_next`, `reduce_next`) rather than generic state machine or helpers. This keeps the fix narrow and ownership-validated.
- Both ForEach and Reduce receive identical fixes since they share the same iteration pattern.
- Push_loop_span finished-pop-before-compare change preserved as it correctly handles sequential loop nesting validation.

## Invariants Preserved
- Generic step state transition table unchanged
- No unsafe, unwrap, expect, panic, todo, unimplemented, or dbg
- No unchecked indexing, slicing, casts, or arithmetic
- No YAML, JSON, or HTTP in runtime core
- Narrow validated ownership: only the loop continuation node resets its own state
- Error propagation through standard ? operator and EngineError
