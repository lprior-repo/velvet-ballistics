# RA-013: Manual `while index < count` + `saturating_add(1)` loops in bounded construction / iteration

- **Severity**: Info
- **Category**: simplification (functional-rust / holzman-rust)
- **Location**:
  - `crates/vb_runtime/src/runtime/runtime_construction.rs:36-40`
  - `crates/vb_runtime/src/runtime/runtime_control.rs:270-277` (`drain_source_commands`)
  - `crates/vb_runtime/src/runtime/runtime_control.rs:319-323` (`collect_shard_summaries`)
  - `crates/vb_runtime/src/runtime/runtime_control.rs:347-363` (`completed_steps`)
- **Confidence**: confirmed

## Description

Multiple call sites use the manual pattern `let mut i = 0; while i < bound { ...; i = i.saturating_add(1); }` for what is idiomatic iterator construction. In every case `bound` is a `usize` that fits in the type, so the `saturating_add` is unreachable defensive code; the body either pushes into a `Vec` or accumulates into a sum.

## Evidence

`runtime_construction.rs:36-40`:

```rust
let count = shard_count.get();
let mut shards = Vec::with_capacity(count);
let mut index = 0usize;
while index < count {
    shards.push(Shard::new_with_journal(config, journal.clone())?);
    index = index.saturating_add(1);
}
```

`runtime_control.rs:270-277`:

```rust
let mut commands = Vec::with_capacity(limit);
let mut drained = 0usize;
while drained < limit {
    let Some(command) = shard.command_queue.pop() else {
        break;
    };
    commands.push(command);
    drained = drained.saturating_add(1);
}
```

`runtime_control.rs:347-363` (`completed_steps`):

```rust
let mut completed = 0u16;
let mut step_index = 0u16;
while step_index < state.workflow.node_count() {
    let step = vb_core::ids::StepIdx::new(step_index);
    if matches!(...) {
        completed = completed.saturating_add(1);
    }
    step_index = step_index.saturating_add(1);
}
completed
```

## Adversarial Check

The `saturating_add(1)` looks defensive against overflow. But every loop bound is the *loop's own exit condition* (`index < count`, `drained < limit`, `step_index < node_count()`), so on the last iteration `index == bound - 1` and `index + 1 == bound ≤ usize::MAX`. Saturating is unreachable. Holzman-rust / functional-rust style treats manual index loops as a code smell when iterator combinators express the same intent more clearly and with stronger exhaustiveness guarantees.

## Suggested Fix

Use iterator combinators:

```rust
// runtime_construction.rs
let shards: Result<Vec<_>, _> = (0..count)
    .map(|_| Shard::new_with_journal(config, journal.clone()))
    .collect();
let shards = shards?;

// drain_source_commands
let commands: Vec<ShardCommand> = (0..limit)
    .map_while(|_| shard.command_queue.pop())
    .collect();

// completed_steps
(0..state.workflow.node_count())
    .filter(|&i| matches!(
        state.frame.step_state(StepIdx::new(i)),
        Ok(StepState::Succeeded) | Ok(StepState::Failed) | Ok(StepState::Skipped) | Ok(StepState::Cancelled)
    ))
    .count() as u16
```
