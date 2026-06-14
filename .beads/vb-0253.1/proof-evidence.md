# Proof Evidence - vb-0253.1

## State 5 Evidence
- Artifact: `crates/vb_runtime/src/kani_shard_command_queue.rs`
- Harness: `command_queue_bounds`
- Symbolic inputs: `capacity: usize`
- Assertions:
  - valid capacity is non-zero
  - valid capacity is at most `MAX_COMMAND_QUEUE_CAPACITY`
  - invalid capacity is exactly zero or above `MAX_COMMAND_QUEUE_CAPACITY`

## Timeout Evidence
- Command attempted: `cargo kani -p vb_runtime --harness command_queue_bounds`
- Result: timed out after 120 seconds while unwinding dependency/drop paths for the earlier `ArrayQueue` mutation harness.
- Raw output: `/home/lewis/.local/share/opencode/tool-output/tool_e3534cabe00116eaY7Co82LaNb`

## Pending Execution
- `rustfmt --edition 2024 crates/vb_runtime/src/kani_shard_command_queue.rs crates/vb_runtime/src/lib.rs crates/vb_runtime/src/shard/types.rs crates/vb_runtime/src/shard/impl_parts/chunk_003.rs crates/vb_runtime/src/shard/impl_parts/chunk_004.rs` -> exit 0.
- `cargo kani -p vb_runtime --harness command_queue_bounds` -> exit 0.
- Kani result: `VERIFICATION:- SUCCESSFUL`; `0 of 3 failed`; `1 successfully verified harnesses, 0 failures, 1 total`. Scope note: this harness runs against the `#[cfg(kani)]` queue model/shared capacity predicate, not production `ArrayQueue` enqueue/dequeue mutation.
