# RS-212-core-shard-command-queue-config-bypass: `from_config` constructs a queue from unvalidated public config capacity

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_runtime/src/shard/queue.rs:56`
- **Confidence**: confirmed

## Description
`ShardCommandQueue::new` rejects zero and over-limit capacities, but `from_config` bypasses that validation and directly builds an `ArrayQueue` from `ShardConfig.command_queue_capacity`. Since `ShardConfig` fields are public, invalid capacities can reach the raw constructor through crate-internal paths.

## Evidence
```rust
40:     pub fn new(capacity: usize) -> RuntimeResult<Self> {
41:         if !is_valid_command_queue_capacity(capacity) {
42:             return Err(crate::RuntimeError::CommandQueueCapacityExceeded {
...
47:         Ok(Self::from_accepted_capacity(capacity))
48:     }
...
56:     pub(crate) fn from_config(config: ShardConfig) -> Self {
57:         Self::from_accepted_capacity(config.command_queue_capacity)
58:     }
...
60:     fn from_accepted_capacity(capacity: usize) -> Self {
61:         Self {
62:             inner: ArrayQueue::new(capacity),
63:             capacity,
64:         }
65:     }
```

`is_valid_command_queue_capacity` is the only guard that enforces `capacity > 0 && capacity <= MAX_COMMAND_QUEUE_CAPACITY`, but `from_config` does not call it. The name `from_accepted_capacity` is not a type-level proof; it receives a plain `usize` copied from a public struct field.

## Adversarial Check
This is not mitigated by the existence of `ShardCommandQueue::new`, because `from_config` does not delegate to it and returns `Self` rather than `RuntimeResult<Self>`. Any crate-internal construction path that uses `from_config` trusts a raw config value that the type system has not validated.

## Suggested Fix
Make `from_config` return `RuntimeResult<Self>` and delegate to `Self::new(config.command_queue_capacity)`, or introduce an `AcceptedShardConfig`/validated capacity newtype so this path cannot be called with unchecked values.
