# Codebase Map — vb-0253.1

**Bead**: vb-0253.1
**Title**: runtime: Wrap shard command queue boundary
**Phase**: State 2 (Explore and Scope)
**Source Checkout**: /home/lewis/src/velvet-ballistics
**Isolated Workspace**: /tmp/vb-ws/vb-0253.1

## Mission

Create a domain-named `ShardCommandQueue` wrapper around `crossbeam_queue::ArrayQueue<ShardCommand>` to isolate direct queue access behind a runtime-owned boundary. Preserve all existing nonblocking full-error behavior.

## Relevant Source Files

### Primary Files

| File | Relevance | Key Findings |
|------|-----------|--------------|
| `crates/vb_runtime/src/shard/types.rs` | PRIMARY | `ArrayQueue<ShardCommand>` directly in `Shard` struct (line 298). `ShardCommand` enum (lines 93-187). `MAX_COMMAND_QUEUE_CAPACITY = 65536` (line 294). `ShardConfig` with `command_queue_capacity` (lines 353-378). |
| `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs` | PRIMARY | `enqueue()` method (lines 44-58) uses `self.command_queue.push(cmd).map_err(\|_\| RuntimeError::QueueFull)`. Queue status methods: `command_queue_len()` (62-64), `remaining_capacity()` (68-72), `is_queue_full()` (76-78), `command_queue_capacity()` (82-84). `tick()` pops from `self.command_queue.pop()` (line 144). |
| `crates/vb_runtime/src/shard/impl_parts/chunk_002.rs` | SECONDARY | `drain_for_shutdown()` uses `self.command_queue.capacity()` (line 59). |
| `crates/vb_runtime/src/shard/mod.rs` | PRIMARY | Re-exports `Shard`, `ShardCommand`, `ShardConfig`, etc. from `types`. Will need to add `ShardCommandQueue` re-export. |
| `crates/vb_runtime/src/error/mod.rs` | SECONDARY | `RuntimeError::QueueFull` (line 7). All error variants defined here. |
| `crates/vb_runtime/src/runtime.rs` | SECONDARY | Multi-shard runtime. Calls `shard.enqueue(ShardCommand::Submit {...})` at line 81 and similar. |

### Test Files (queue behavior preservation)

| File | Key Tests |
|------|-----------|
| `crates/vb_runtime/src/shard/tests/chunk_011.rs` | `shard_command_queue_len_starts_at_zero` (254), `shard_command_queue_len_increments_on_enqueue` (263). Queue behavior tests. |
| `crates/vb_runtime/src/shard/tests/chunk_012.rs` | `shard_remaining_capacity_decrements_on_enqueue` (3), `shard_is_queue_full_returns_false_initially` (40), `shard_is_queue_full_returns_true_when_at_capacity` (49), `shard_command_queue_capacity_returns_configured_value` (67). Config validation tests (125-179). |
| `crates/vb_runtime/src/shard/tests/chunk_025.rs` | Queue overflow test (line 169: `Err(RuntimeError::QueueFull)`). Direct `command_queue.len()` access. |
| `crates/vb_runtime/src/shard/tests/chunk_026.rs` | Queue full test (line 16: `Err(RuntimeError::QueueFull)`). |
| `crates/vb_runtime/src/shard/impl_tests/chunk_001.rs` | Queue capacity/behavior tests (lines 148-190). |
| `crates/vb_runtime/src/shard/impl_tests/chunk_002.rs` | Queue depth in status (lines 116-134). |

## Architecture Context

### Master Document (velvet-ballistics-MASTER.md)

- **Line 209**: `crossbeam-queue::ArrayQueue` for bounded MPMC shard queues. No unbounded channel replacement.
- **Line 225**: `ArrayQueue` capacity fixed at construction; admission can fail without allocating.
- **Line 991**: "Bounded inbound command queue using `crossbeam_queue::ArrayQueue`." (Section 20)
- **Line 247**: "Queues and scheduling use bounded `ArrayQueue`/`rtrb`"

### Parent Epic

- **vb-0253**: "architecture: Standardize queue and state boundaries"
  - No unbounded queue introduction
  - Domain wrappers remain local (no generic queue abstraction soup)
  - Queue/state meaning must not be erased

## Current Behavior to Preserve

1. **enqueue**: Returns `Ok(())` on success, `Err(RuntimeError::QueueFull)` when full. No blocking, no allocating on full.
2. **command_queue_len**: Returns current depth.
3. **remaining_capacity**: Returns `capacity() - len()` (saturating).
4. **is_queue_full**: Returns true when at capacity.
5. **command_queue_capacity**: Returns fixed bounded capacity set at construction.
6. **tick**: Pops at most ONE command per call (FIFO order).
7. **drain_for_shutdown**: Processes all queued commands up to capacity.
8. **status**: Reports `command_queue_depth` and `command_queue_capacity`.

## Risk Tags

- **public_api**: New wrapper type added to public exports; existing API must remain compatible.
- **performance**: Queue is on hot path; wrapper must not add measurable overhead.
- **concurrency**: `ArrayQueue` is already thread-safe for MPMC; wrapper does not change this.
- **no_unsafe**: `#![forbid(unsafe_code)]` applies; no unsafe needed.

## Open Questions

1. **Wrapper name**: `ShardCommandQueue` is the domain name per the bead description. Confirm this is the intended public-facing name.
2. **Internal API**: Should `Shard` access the wrapper via `pub(crate)` methods or directly hold and use the wrapper?
3. **Test impact**: Tests in `chunk_025.rs` and `chunk_026.rs` use `shard.command_queue.len()` directly — these will need updating if direct field access is replaced with method access.

## Recommended Downstream Owners

- **State 3 (Contract)**: `rust-contract` for requirements formalization.
- **State 7 (Test)**: `test-planner` for queue behavior test plan.
- **State 10 (Implementation)**: `holzman-rust` for safe Rust implementation.
