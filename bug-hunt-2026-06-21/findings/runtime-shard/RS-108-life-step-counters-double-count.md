# RS-108-life: Lifecycle counters add cumulative executed steps repeatedly

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/transitions/continuation.rs:42`
- **Confidence**: confirmed

## Description

Continuation and terminal paths add `state.frame.executed()` to the shard counters every time the run suspends or terminates. The surrounding snapshot code treats `executed()` as a cumulative run counter, so multi-suspension runs are overcounted.

## Evidence

`keep_run_with_snapshot` reads cumulative execution and adds it directly:

```rust
let executed = state.frame.executed();
let interval = self.snapshot_interval_steps;
let last_executed = state.last_snapshot_executed;
...
self.counters.add_steps(executed);
```

Other paths do the same, for example `await_action` at `continuation.rs:67`, `await_timer` at `continuation.rs:106`, and `finish_run` at `terminal.rs:25-26`:

```rust
self.counters.inc_completed();
self.counters.add_steps(state.frame.executed());
```

No code in these paths resets the frame execution count or records a last-accounted value after adding it.

## Adversarial Check

This is not assuming an external counter semantic. `last_snapshot_executed` is compared against `executed`, which shows `executed()` is being used as a monotonic per-run total rather than a per-drive delta. Adding that same total after each suspension and again at terminal completion necessarily double-counts any steps already reported.

## Suggested Fix

Track `last_accounted_executed` in `RunState` and add only the delta with checked arithmetic, or reset the frame's accounted step count after each successful counter update. Keep snapshot watermarking separate from metrics accounting so they cannot reuse a cumulative value accidentally.
