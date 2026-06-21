# RS-216-core-unbounded-shard-command-payloads: Queue capacity bounds command count but not command payload size

- **Severity**: Medium
- **Category**: perf
- **Location**: `crates/vb_runtime/src/shard/command.rs:37`
- **Confidence**: confirmed

## Description
`ShardCommand` variants expose unbounded heap payloads while `ShardCommandQueue` only bounds the number of queued commands. A single command can carry an arbitrarily large input list, contract list, workflow, or cancellation reason, defeating the shard's bounded-resource model.

## Evidence
```rust
37:     SubmitWithInputs {
38:         /// Run identifier chosen by the caller.
39:         run: RunId,
40:         /// Compiled workflow to execute.
41:         workflow: CompiledWorkflow,
42:         /// Initial slot values written before deterministic execution starts.
43:         inputs: Box<[(SlotIdx, SlotValue)]>,
...
56:         action_contracts: Box<[ActionContract]>,
...
65:         inputs: Box<[(SlotIdx, SlotValue)]>,
...
69:         action_contracts: Box<[ActionContract]>,
...
120:     Cancel {
...
125:         reason: Option<String>,
126:     },
127:     /// Kill an active run unconditionally.
128:     Kill {
...
132:         reason: Option<String>,
133:     },
```

The queue limit caps entries, not bytes. With `MAX_COMMAND_QUEUE_CAPACITY` at 65,536, the worst-case memory footprint is still unbounded because each command can own arbitrary heap data.

## Adversarial Check
This is not solved by enqueue failure on a full queue. The payload is already allocated before enqueue, and the accepted command can remain in the bounded queue while consuming unbounded memory. The enum is public and its fields are public, so the type itself does not enforce any maximum.

## Suggested Fix
Introduce bounded command payload newtypes or checked constructors for input slices, action-contract slices, workflows, and reason strings. Keep enum fields private or crate-private so all command construction passes through size validation.
