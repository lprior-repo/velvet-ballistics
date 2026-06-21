# RE-011: Drive step state and evidence updates are not transactional

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/engine/drive.rs:105-118`
- **Confidence**: confirmed

## Description

The drive loop records or commits step state before later fallible operations complete. A failed `mark_running` can leave a `StepStarted` evidence event for a step that never started, and a failed slot-evidence emission can leave a step marked succeeded even though `drive_deterministic_full` returns an error.

## Evidence

`crates/vb_runtime/src/engine/drive.rs:105-106` emits evidence before the state transition succeeds:

```rust
evidence.push_step_started(pc);
run.mark_running(pc).map_err(RuntimeEngineError::Core)?;
```

`crates/vb_runtime/src/engine/drive.rs:117-118` commits the post-signal state before fallible evidence capture:

```rust
mark_step_after_signal(run, step.pc, signal).map_err(RuntimeEngineError::Core)?;
emit_slot_evidence(run, evidence, collect_states, step.node)?;
```

`emit_slot_evidence` can return an error through `run.read_taint(...)?` or through `push_slot_written_with_extra(...)?` when collect evidence with extra data exceeds capacity. In that case the run frame has already been mutated by `mark_step_after_signal`, so callers receive an error while the frame may show the step as succeeded.

## Adversarial Check

This is not just an instrumentation nit. The code uses `?` on `mark_running`, `mark_step_after_signal`, `read_taint`, and `push_slot_written_with_extra`, so these operations are explicitly fallible. Because the mutations happen before later fallible work, the evidence stream and frame state can describe a step that did not commit cleanly. Normal happy paths will not expose it, but capacity exhaustion and frame/slot errors are exactly the cases where the audit trail must be conservative.

## Suggested Fix

Make each drive step commit in one direction. For step start, call `mark_running` before `push_step_started`. For step finish, collect all fallible slot evidence first, then mark the step and push `StepSucceeded`; or stage evidence in a temporary value and append it only after all state transitions succeed.
