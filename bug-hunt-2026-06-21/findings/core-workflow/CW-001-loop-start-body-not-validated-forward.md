# CW-001: `validate_loop_done_only` skips forward-edge validation of `body` for loop-start variants

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_core/src/workflow/validation/forward_edges.rs:130-137` (call sites at `forward_edges.rs:69-93`)
- **Confidence**: confirmed

## Description

`validate_kind_edges` delegates all loop variants (ForEachStart/Next, CollectStart/Page/Next, ReduceStart/Next, RepeatStart/Attempt, RetryCheck, ErrorHandler) to `validate_loop_done_only`, whose signature explicitly drops the body parameter (`_body: StepIdx`). For `*Start` variants the `body` field semantically points forward (the first instruction of the loop body), but the validator never enforces this — only `done` is checked.

## Evidence

```rust
// forward_edges.rs:130
fn validate_loop_done_only(
    _body: StepIdx,
    done: StepIdx,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    validate_forward_target(done, ci, cid)
}
```

Call sites (forward_edges.rs:69-93) cover `ForEachStart`, `CollectStart`, `ReduceStart`, `RepeatStart` whose `body` should be a forward edge (entry into the loop body). A compiler bug or adversarial input could emit `RepeatStart { body: <self>, done: <next> }` and pass this validator.

## Adversarial Check

The runtime back-edge cycle detector in `budget/traversal_step_count.rs:164-181` (JumpCycle detection) plus the `count_and_push_loop_body` traversal catches unbounded loops via Jump, and the per-tick step budget (`MAX_STEP_BUDGET`) caps any runaway. So this is not an unbounded loop DoS — it is a contract gap where validation under-specifies the IR shape for `*Start` variants. The `*Next` variants legitimately need backward `body` (to re-enter the loop), so the shared helper exists for them; the `*Start` variants should not share it.

## Suggested Fix

Split the helper: keep `validate_loop_done_only` for `*Next` variants and add `validate_loop_start_edges` for `*Start` variants that calls `validate_forward_target` for both `body` and `done`. This matches the contract already enforced at runtime where `*Start.body` is entered once via a forward jump.
