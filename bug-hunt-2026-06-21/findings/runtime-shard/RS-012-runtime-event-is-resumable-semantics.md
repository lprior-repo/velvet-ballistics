# RS-012: `RuntimeEvent::is_resumable` returns true for `Resume`, but `Resume` transitions to `Resuming`, not `Resumable`

- **Severity**: Low
- **Category**: correctness / semantic bug
- **Location**: `crates/vb_runtime/src/shard/run_state.rs:161-176`
- **Confidence**: confirmed

## Description

The doc on `RuntimeEvent::is_resumable` says "Returns true if this event sets a Resumable state." The impl matches `AwaitAction | AwaitTimer | Resume`. But `Resume` actually transitions to `Resuming` (per the FSM at `transitions/fsm.rs:50-52`), not `Resumable`. The predicate lies for one of its three cases.

## Evidence

```rust
// run_state.rs:161-176
impl RuntimeEvent {
    /// Returns true if this event is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Fail | Self::TerminalRemove | Self::DriveFinished)
    }

    /// Returns true if this event sets a Resumable state.
    pub fn is_resumable(&self) -> bool {
        matches!(self, Self::AwaitAction | Self::AwaitTimer | Self::Resume)
    }
}
```

```rust
// transitions/fsm.rs:45-70
pub(crate) fn apply(&mut self, run: RunId, event: RuntimeEvent) {
    match event {
        …
        RuntimeEvent::Resume => {
            self.runtime_state_insert(run, RuntimeState::Resuming);   // ← not Resumable
        }
        RuntimeEvent::ResumeRollback => {
            self.runtime_state_insert(run, RuntimeState::Resumable);
        }
        RuntimeEvent::AwaitAction | RuntimeEvent::AwaitTimer => {
            self.runtime_state_insert(run, RuntimeState::Resumable);
        }
        …
    }
}
```

The actual "event sets Resumable state" set is `{AwaitAction, AwaitTimer, ResumeRollback}`. The impl incorrectly includes `Resume` and incorrectly excludes `ResumeRollback`.

The `RuntimeState::is_resumable` predicate on the same file (lines 128-134) is correctly narrow:

```rust
pub fn is_resumable(&self) -> bool {
    matches!(self, Self::Resumable)
}
```

So at the state level the predicate is correct, but at the event level it disagrees with itself.

## Adversarial Check

A defender might argue `Resume` "preserves resumability" because the run was Resumable before. But the predicate is named "sets a Resumable state" (post-condition), not "preserves". After `Resume` the state is `Resuming`, full stop. If a caller uses `event.is_resumable()` to decide whether to register a wake-up for the run, a `Resume` event would cause an incorrect wake-up registration on a run that is in `Resuming` state (which is not, per `RuntimeState::is_resumable`, resumable).

This file is in the verification-binding surface (`ipc_refinement.rs:148-158` calls `event.is_terminal()` and `state.is_resumable()`). If a future Flux refinement uses `event.is_resumable()` as a witness, the proof will be vacuous for the `Resume` case.

## Suggested Fix

```rust
/// Returns true if this event sets a Resumable state.
pub fn is_resumable(&self) -> bool {
    matches!(
        self,
        Self::AwaitAction | Self::AwaitTimer | Self::ResumeRollback
    )
}
```
