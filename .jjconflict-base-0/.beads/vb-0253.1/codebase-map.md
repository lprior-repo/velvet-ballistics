# Codebase Map - vb-0253.1

## Bead
**ID**: vb-0253.1  
**Title**: Wrap shard command queue boundary

## Scope

### Relevant Crates/Files
| Path | Role | Risk Tags |
|------|------|-----------|
| crates/vb_runtime/src/shard/types.rs | Shard command queue types and config | persistence, public-api |
| crates/vb_runtime/src/shard/impl_parts/chunk_001.rs | Command queue processing logic | concurrency |
| crates/vb_runtime/src/shard/tests/chunk_010.rs | Command queue tests | test |
| crates/vb_runtime/src/shard/tests/chunk_011.rs | Command queue length tests | test |
| crates/vb_runtime/src/shard/tests/chunk_012.rs | Command queue capacity tests | test |
| crates/vb_runtime/src/shard/tests/chunk_025.rs | Command queue full scenario tests | test |
| crates/vb_runtime/src/shard/tests/chunk_026.rs | Command queue pending timers tests | test |
| crates/vb_runtime/src/shard/impl_tests/chunk_001.rs | Command queue impl tests | test |
| crates/vb_runtime/src/shard/impl_tests/chunk_002.rs | Command queue impl tests | test |

### Key Symbols
- `ShardConfig::command_queue_capacity()`
- `shard.command_queue_len()`
- `shard.command_queue_capacity()`
- `shard.command_queue`
- `shard.enqueue_command()`

### Public API Surface
- `vb_runtime::shard::types::ShardConfig`
- `vb_runtime::shard::Shard`

### Risk Tags
- **concurrency**: Command queue is shared between shard threads
- **persistence**: Queue state may need recovery
- **public-api**: Shard configuration exposed to runtime

### Open Questions
- What is the exact boundary being wrapped?
- Are there existing Kani proofs for command queue transitions?
- Is there a need for formal verification of queue bounds?

### Excluded Paths (Not in Scope)
- vb_ipc ingress code (separate bead vb-0253.2)
- vb_proof_kernels StepState (separate bead vb-0253.5)

### Downstream Owners
- rust-contract: needs to define queue boundary contract
- proof-planner: may need Kani proofs for concurrent access
- holzman-rust: implementation if code changes needed
