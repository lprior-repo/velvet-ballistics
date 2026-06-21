# CW-004: `push_loop_span` rejects valid sequential loops by checking stale spans before popping them

- **Severity**: High
- **Category**: bug
- **Location**: `crates/vb_core/src/workflow/validation/forward_edges.rs:192-217`
- **Confidence**: confirmed

## Description

The loop-nesting validator checks a new loop against the last recorded span before removing spans that have already ended. This rejects ordinary sequential loops when the second loop's `done` is after the first loop's `done`.

## Evidence

```rust
// forward_edges.rs:192
match spans.last().copied() {
    Some((_outer_start, outer_done)) if done_idx > outer_done => {
        return Err(WorkflowError::ImproperLoopNesting { ... });
    }
    _ => {}
}

while spans
    .last()
    .is_some_and(|&(_, done): &(usize, usize)| done <= ci)
{
    spans.pop();
}

spans.push((ci, done_idx));
```

A loop start at index `1` with `done = 3` pushes span `(1, 3)`. A separate later loop start at index `4` with `done = 6` hits the match first, sees `done_idx > outer_done` (`6 > 3`), and returns `ImproperLoopNesting`. The stale span would have been popped by the immediately following `while done <= ci` check because `3 <= 4`.

## Adversarial Check

This is not a disputed inclusive/exclusive boundary interpretation: the function itself says ended spans are stale once `done <= ci`, but it performs that cleanup after the nesting comparison. Sequential loops are a normal valid workflow shape and should not be constrained by a previously closed loop span.

## Suggested Fix

Move the `while spans.last().is_some_and(|(_, done)| done <= ci)` cleanup before the `done_idx > outer_done` nesting check. Then compare the new span only against active enclosing spans.
