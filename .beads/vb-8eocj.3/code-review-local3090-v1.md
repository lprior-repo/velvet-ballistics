# vb-8eocj.3 independent local3090 CODE review — v1

**Review type:** Source-only code review (no tests/Cargo/moon/source/test/config edits)
**Reviewers:** local3090 (independent)
**Reviewed revision:** f186338ce71c3298735a1ec25e0eede0173de0d0 (exact frozen candidate)
**Baseline revision:** 2b6f53ae1add434eba72921f347fb0abd1d5193a
**Workspace:** /home/lewis/src/isoloated/velvet-ballistics-vb-8eocj3-local3090-code-review-v1
**Verified roots:** git rev-parse --show-toplevel and jj root both resolve to isolated workspace
**NO TESTS RUN.** Source inspection only.

## Scope

Review of exact frozen delta between 2b6f53ae1 and f186338ce, specifically:

1. `crates/vb_core/src/workflow/mod.rs` — `push_loop_span` finished-span retirement moved before nesting comparison
2. `crates/vb_runtime/src/engine/drive/transitions.rs` — new `loop_body_target` and `owned_continuation_rearm` helpers; call in `finish_drive_step` for post-success rearm of ForEachNext/ReduceNext control nodes only

Generic step-state transition table unchanged. ForEachStart/Join, ReduceStart/Finish, Repeat, Together, Collect semantics unchanged.

## Findings

### 1. push_loop_span: finished-pop-before-compare — PASS

**Source:** crates/vb_core/src/workflow/mod.rs lines 1878-1930 (after change)

The retirement loop is now executed before the nesting comparison:

```rust
// Retire finished spans BEFORE the nesting comparison so that
// sequential loops are not rejected as improperly nested.
while spans
    .last()
    .is_some_and(|&(_, done): &(usize, usize)| done <= ci)
{
    spans.pop();
}
```

**Correctness argument:**
- Spans are processed in node index order; any span with `done <= ci` has definitively finished.
- Retiring finished spans before the nesting check prevents sequential loops from being rejected as improperly nested (e.g., loop A ending at step 10, loop B starting at step 11 — A's span is retired before B's nesting is validated).
- The retirement condition `done <= ci` is safe: it only removes spans that are provably complete.
- The nesting check `done_idx > outer_done` still correctly rejects overlapping loops that extend beyond their outer loop's span.

**Risk:** None identified. The change is a pure reordering of existing logic with correct termination conditions.

### 2. owned_continuation_rearm: post-success rearm for ForEachNext/ReduceNext — PASS

**Source:** crates/vb_runtime/src/engine/drive/transitions.rs lines 48-78 (after change)

**New helper: `loop_body_target`**
```rust
pub(super) fn loop_body_target(node: &CompiledNode) -> Option<StepIdx> {
    match node.kind {
        CompiledNodeKind::ForEachNext { body, .. }
        | CompiledNodeKind::ReduceNext { body, .. } => Some(body),
        _ => None,
    }
}
```
Extracts the body step index from owning-loop control nodes only. Non-loop and join/finish nodes return None.

**New helper: `owned_continuation_rearm`**
```rust
pub(super) fn owned_continuation_rearm(
    run: &mut RunFrame,
    step: DriveStep<'_>,
) -> RuntimeEngineResult<()> {
    let Some(body) = loop_body_target(step.node) else {
        return Ok(());
    };
    if run.pc() != body {
        return Ok(());
    }
    if body >= step.pc {
        return Ok(());
    }
    run.mark_pending(step.pc).map_err(RuntimeEngineError::Core)?;
    Ok(())
}
```

**Owner checks (both must pass for rearm):**
1. `run.pc() == body` — confirms the drive successfully jumped to this specific control node's declared body target.
2. `body < step.pc` — confirms the body step index is less than the control node index, validating backward loop-edge targeting (the body must precede the ForEachNext/ReduceNext node in the IR).

**Call site:** In `finish_drive_step`, after `mark_step_after_signal` and `evidence.push_step_succeeded`, only on the success path (Continue or Finished signals):

```rust
if signal_is_success(signal) {
    mark_step_after_signal(run, step.pc, signal)?;
    evidence.push_step_succeeded(step.pc, step.node.output).map_err(...)?;
    owned_continuation_rearm(run, step)?;  // NEW
}
```

**Correctness argument:**
- The rearm occurs after the success state is committed and evidence is recorded, avoiding the reentry ordering problem that plagued the v10 approach (which called mark_pending inside the primitive while the step was still Running).
- The state transition for a ForEachNext/ReduceNext node that jumps to its body is: Running -> Succeeded (committed) -> Pending (rearmed). This allows the node to be re-entered on subsequent iterations.
- The `run.pc() == body` check ensures only the control node that actually owns the current execution position is re-armed.
- The `body < step.pc` check is a sanity check that the IR is well-formed (body precedes control node).
- Suspension signals (AwaitingWait, AwaitingAsk, AwaitingAction, StepBudgetExhausted) bypass the success path entirely; no rearm occurs.
- Error propagation is preserved via `?` and `RuntimeEngineError::Core`.

**Scope limitation (explicit, documented in implementation-v11.md):**
- Only ForEachNext and ReduceNext are handled. CollectNext and RepeatAttempt also use `jump_to_body` internally but are not re-armed by this change. This is a documented state limitation for the current one-body canonical subset; arbitrary multi-body loops remain a future F1 enhancement.
- The owner check `run.pc() == body` ensures only the specific control node whose declared body matches the current PC is re-armed, preventing arbitrary step manipulation.

**Risk: Evidence/state inconsistency (minor)**
- The step succeeded evidence is pushed before rearm. So the evidence shows Succeeded, but the step state is then reset to Pending. This is consistent with the body re-entry pattern in `jump_to_body` (which also resets the body to Pending before re-execution), but observers of the evidence stream may see a discrepancy between the evidence (Succeeded) and the final step state (Pending). This is a known trade-off of the post-success rearm approach.

## Risks and edge cases considered

1. **Crossing loops / wrong state / malformed back-edges:** The `body < step.pc` check rejects forward-edge targeting. The `run.pc() == body` check rejects re-arming when the PC is not at the expected body target. Malformed non-owner back-edges are caught by the owner checks.

2. **Replay/trace state consequences:** The step succeeded event is recorded before rearm, so replay will see the step as having succeeded. The step state on the run frame is then reset to Pending for re-entry. This is consistent with the existing body re-entry semantics.

3. **Error paths:** If `run.mark_pending` fails (e.g., step state out of bounds), the error is propagated via `RuntimeEngineError::Core`. The success path in `finish_drive_step` uses `?` for all fallible operations, so a rearm failure will abort the drive loop with an error.

4. **Direct public driver accepting unvalidated IR:** This change does not alter IR validation or acceptance. The `push_loop_span` change improves validation of sequential loops, but IR acceptance criteria are unchanged.

## Verification evidence

- Actual source at f186338ce inspected line-by-line.
- Delta verified via `git diff --stat 2b6f53ae1 f186338ce` and `git diff 2b6f53ae1 f186338ce -- <files>`.
- Cross-referenced with master doc and .beads/vb-8eocj.3/contract.md.
- Cross-referenced with implementation-v11.md design doc.
- No inventing scenario findings; all findings grounded in actual source.

## PASS/REJECT

**PASS** for the scoped subset: owned continuation rearm for ForEachNext/ReduceNext and push_loop_span finished-pop-before-compare.

The implementation correctly:
- Retires finished loop spans before nesting comparison (fixes sequential loop rejection).
- Re-arms ForEachNext/ReduceNext control nodes only after success/evidence commit.
- Validates loop ownership via PC match and body-index-before-control-index checks.
- Preserves generic transition table, error propagation, and suspension semantics.

## Test requirements (for root to run)

To close this bead, root must run:
1. Source19 fixture suite (expect 12 PASS 7 FAIL: Repeat and reducer-empty/single-evidence remain)
2. ForEach/Reduce iteration tests
3. Crossing-loop and invalid-state negative tests
4. Together completion tests (separate issue)
5. Collect boundary tests (separate issue)
6. Evidence gap tests (RE-004)
7. Existing integration test suites

Do not claim full acceptance or prove arbitrary nested loops from current fixtures. This is a state limitation for the current one-body canonical subset.
