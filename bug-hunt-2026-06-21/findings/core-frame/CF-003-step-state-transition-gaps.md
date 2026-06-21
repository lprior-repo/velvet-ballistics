# CF-003: `is_valid_step_state_transition` omits `Waiting → Cancelled`, `Asking → Cancelled`, and `Succeeded → Pending`

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_core/src/frame/step_state.rs:38`
- **Confidence**: confirmed

## Description

The pure transition predicate `is_valid_step_state_transition` does not
permit:
- `Waiting → Cancelled` or `Asking → Cancelled` — but a workflow that is
  suspended on a wait/ask primitive must be cancellable by an external
  timeout or cancel signal without first resuming to `Running`.
- `Succeeded → Pending` — yet `RunFrame::mark_pending` allows exactly
  this transition via the separate `validate_pending_admission` path
  (`frame/transitions.rs:115-122`), and the module docstring
  (`step_state.rs:30-36`) advertises it as the loop-body re-entry path.

This split makes the predicate unreliable for proof harnesses (which use
it as the canonical contract) and forces the runtime to bypass it for
loop re-entry.

## Evidence

```rust
pub fn is_valid_step_state_transition(current: StepState, new: StepState) -> bool {
    if current == new {
        return true;
    }
    matches!(
        (current, new),
        (StepState::Pending, StepState::Running)
            | (StepState::Pending, StepState::Succeeded)
            | ...
            | (StepState::Waiting, StepState::Running)
            | (StepState::Asking, StepState::Running)
    )
}
```

(`crates/vb_core/src/frame/step_state.rs:38-58`)

No `Waiting → Cancelled`, no `Asking → Cancelled`, no `Succeeded → Pending`.

Meanwhile:

```rust
fn validate_pending_admission(current: StepState) -> CoreResult<()> {
    match current {
        StepState::Pending | StepState::Succeeded => Ok(()),   // <-- bypasses predicate
        ...
    }
}
```

(`crates/vb_core/src/frame/transitions.rs:115-122`)

## Adversarial Check

A defender might claim "the predicate is intentionally narrow and
`mark_pending` is the documented escape hatch." But the predicate's own
docstring (`step_state.rs:30-36`) says it is the "shared" contract used
by "runtime validation and proof harnesses." Verus proofs keyed on this
predicate will *prove* that loop re-entry is impossible — a vacuous
result that hides a real runtime capability. Likewise, a Kani harness
checking cancellation of a `Waiting` step will conclude the transition
is always rejected, which is operationally wrong.

## Suggested Fix

Add the missing transitions to `is_valid_step_state_transition` so the
predicate matches reality. If `Waiting → Cancelled` is genuinely
forbidden by Phase-0 semantics, document it and audit the cancellation
path to ensure it never happens; otherwise cancellation code is silently
broken.
