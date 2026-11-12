# Contract Specification - vb-0253.1

## Context
- **Feature**: Wrap shard command queue boundary
- **Domain terms**: Shard, Command Queue, Queue Capacity, Enqueue, Dequeue
- **Assumptions**: Command queue is bounded, multi-producer single-consumer within shard
- **Open questions**: What is the exact boundary being wrapped? Is this about wrapping the queue API or the capacity boundary?

## Preconditions
- PRE-001: Shard must be initialized with a valid command queue capacity > 0
- PRE-002: Enqueue operation requires shard to be in Running state
- PRE-003: Command payload must not exceed configured max size

## Postconditions
- POST-001: After enqueue, `command_queue_len()` reflects the new length
- POST-002: After successful enqueue, the command is available for processing
- POST-003: When queue is full, enqueue returns an error and does not modify queue state
- POST-004: Dequeue reduces `command_queue_len()` by exactly 1

## Invariants
- INV-001: `command_queue.len() <= command_queue.capacity()` always holds
- INV-002: `command_queue_len()` == `command_queue.len()` always holds
- INV-003: Queue capacity is fixed after shard initialization

## Error Taxonomy
- Error::QueueFull - when command queue is at capacity
- Error::InvalidState - when enqueue attempted on non-Running shard
- Error::PayloadTooLarge - when command exceeds max payload size

## Contract Signatures
- `fn enqueue_command(cmd: Command) -> Result<(), QueueError>`
- `fn dequeue_command() -> Result<Option<Command>, QueueError>`
- `fn command_queue_len() -> usize`
- `fn command_queue_capacity() -> usize`

## TLA+-Owned Clauses
- None identified - queue operations are local to a shard, no cross-shard temporal behavior specified

## Verus-Owned Clauses
- INV-001: Queue length never exceeds capacity - proven by Verus invariant
- INV-002: Length accessor matches actual queue length
- PRE-001, PRE-002, PRE-003: Preconditions enforced by Verus-spec'd constructors

## Non-goals
- Cross-shard queue communication
- Distributed queue protocols
- Async queue operations with timeouts
