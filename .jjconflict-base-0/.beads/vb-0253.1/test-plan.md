# Test Plan - vb-0253.1

STATUS: APPROVED

## Required Scenarios
- Given zero command queue capacity, when `ShardConfig::new` is called, then `CommandQueueCapacityExceeded` is returned.
- Given capacity above `MAX_COMMAND_QUEUE_CAPACITY`, when `ShardConfig::new` is called, then `CommandQueueCapacityExceeded` is returned.
- Given boundary capacities, when `is_valid_command_queue_capacity` is called, then it accepts exactly `1..=MAX_COMMAND_QUEUE_CAPACITY`.
- Given a bounded shard queue, when commands are enqueued to capacity, then length never exceeds capacity and further enqueue returns `QueueFull`.
- Given queued commands, when `tick` consumes a command, then `command_queue_len()` decreases.

## Commands
- `cargo test -p vb_runtime command_queue -- --nocapture`
- `cargo check -p vb_runtime`
