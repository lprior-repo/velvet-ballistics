# CE-004: Replay maps invalid jump targets to a generic internal error

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/replay/basic/handlers/mod.rs:49`
- **Confidence**: confirmed

## Description

Replay exposes `ReplayError::StepNotFound`, but invalid jump/branch targets produced by `run.set_pc` are passed through a slot-only converter and collapse to `ReplayError::Internal`. Corrupt target evidence loses the offending step index.

## Evidence

The shared converter only preserves slot errors:

```rust
pub(super) fn slot_to_replay_err(e: EngineError) -> ReplayError {
    match e {
        EngineError::SlotOutOfBounds { slot } => ReplayError::SlotNotAvailable { slot },
        EngineError::SlotUninitialized { slot } => ReplayError::SlotNotAvailable { slot },
        _ => ReplayError::Internal {
            reason: "unexpected engine error during replay",
        },
    }
}
```

Jump replay uses that converter for program-counter validation:

```rust
run.set_pc(target).map_err(shared::slot_to_replay_err)?;
```

Choose replay does the same for branch targets:

```rust
run.set_pc(branch.target).map_err(slot_to_replay_err)?;
```

## Adversarial Check

Workflow validation should prevent bad targets in accepted artifacts, but replay is explicitly a corruption/reconstruction boundary. The enum already has `StepNotFound { step }` and `replay_one` uses it when the current node is missing. Losing the same information for bad transition targets is a real observability and recovery bug, not an intentional validation shortcut.

## Suggested Fix

Replace the slot-only converter with a replay-wide `engine_to_replay_err` that maps `EngineError::InvalidProgramCounter { step }` to `ReplayError::StepNotFound { step }`, maps slot errors to `SlotNotAvailable`, and reserves `Internal` for genuinely unexpected errors.
