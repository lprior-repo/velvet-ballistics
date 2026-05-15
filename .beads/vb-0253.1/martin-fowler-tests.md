# Martin Fowler Test Plan — vb-0253.1

## Overview

`ShardCommandQueue` is a bounded non-blocking command queue wrapper over `crossbeam_queue::ArrayQueue<ShardCommand>`. Tests cover: constructor, enqueue success/failure, pop success/empty, status method consistency, FIFO ordering, and tick-at-most-one behavior at the `Shard` level.

## Happy Path Tests

### test_enqueue_succeeds_when_queue_has_capacity
**Given**: A `ShardCommandQueue` constructed with `capacity = 4`
**When**: `enqueue(SpawnActor)` is called
**Then**: Returns `Ok(())`, `len()` increases by 1, `remaining_capacity()` decreases by 1

### test_pop_returns_enqueued_command_fifo
**Given**: A `ShardCommandQueue` with `capacity = 4` containing `[cmd_A, cmd_B]` (A enqueued first)
**When**: `pop()` is called
**Then**: Returns `Some(cmd_A)` (oldest first); subsequent `pop()` returns `Some(cmd_B)`

### test_pop_returns_none_when_queue_is_empty
**Given**: An empty `ShardCommandQueue`
**When**: `pop()` is called
**Then**: Returns `None`

### test_len_returns_zero_for_new_queue
**Given**: A newly constructed `ShardCommandQueue`
**When**: `len()` is called
**Then**: Returns `0`

### test_capacity_returns_configured_value
**Given**: A `ShardCommandQueue` constructed with `capacity = 1024`
**When**: `capacity()` is called
**Then**: Returns `1024`

### test_remaining_capacity_decrements_on_enqueue
**Given**: A `ShardCommandQueue` with `capacity = 10`
**When**: `enqueue(cmd)` is called 3 times
**Then**: `remaining_capacity()` returns `7`

### test_is_full_returns_false_for_non_full_queue
**Given**: A `ShardCommandQueue` with `capacity = 4` containing 2 elements
**When**: `is_full()` is called
**Then**: Returns `false`

### test_bounded_capacity_returns_max
**Given**: Any `ShardCommandQueue`
**When**: `ShardCommandQueue::bounded_capacity()` is called
**Then**: Returns `65536`

## Error Path Tests

### test_enqueue_returns_queue_full_when_at_capacity
**Given**: A `ShardCommandQueue` with `capacity = 2` already containing `[cmd_A, cmd_B]`
**When**: `enqueue(cmd_C)` is called
**Then**: Returns `Err(RuntimeError::QueueFull)`

### test_enqueue_does_not_modify_queue_state_on_queue_full
**Given**: A `ShardCommandQueue` with `capacity = 2` containing `[cmd_A, cmd_B]`
**When**: `enqueue(cmd_C)` is called and returns `Err(RuntimeError::QueueFull)`
**Then**: `len()` is still `2`, `remaining_capacity()` is still `0`, `is_full()` is still `true`

### test_enqueue_does_not_block_and_does_not_allocate_on_full
**Given**: A `ShardCommandQueue` at capacity
**When**: `enqueue(cmd)` is called in a tight loop 1000 times
**Then**: All calls return `Err(RuntimeError::QueueFull)` immediately; no allocation occurs

### test_try_new_returns_invalid_configuration_when_capacity_zero
**Given**: `capacity = 0`
**When**: `ShardCommand::try_new(0)` is called
**Then**: Returns `Err(RuntimeError::InvalidConfiguration)`

### test_try_new_returns_invalid_configuration_when_capacity_exceeds_max
**Given**: `capacity = 65537`
**When**: `ShardCommand::try_new(65537)` is called
**Then**: Returns `Err(RuntimeError::InvalidConfiguration)`

## Edge Case Tests

### test_pop_single_element_queue
**Given**: A `ShardCommandQueue` with exactly 1 element
**When**: `pop()` is called
**Then**: Returns `Some(cmd)`; subsequent `pop()` returns `None`; `len()` goes to `0`

### test_enqueue_pop_interleaved
**Given**: A `ShardCommandQueue` with `capacity = 4`
**When**: enqueue A, pop, enqueue B, pop, enqueue C, pop are called in sequence
**Then**: Pops return `Some(A)`, `Some(B)`, `Some(C)` respectively; queue is empty at end

### test_queue_at_capacity_is_full
**Given**: A `ShardCommandQueue` with `capacity = 3` containing `[cmd_A, cmd_B, cmd_C]`
**When**: `is_full()` is called
**Then**: Returns `true`

### test_remaining_capacity_zero_when_full
**Given**: A `ShardCommandQueue` at capacity
**When**: `remaining_capacity()` is called
**Then**: Returns `0`

### test_multiple_enqueues_then_multiple_pops
**Given**: A `ShardCommandQueue` with `capacity = 4`
**When**: 4 commands are enqueued then 4 pops are executed
**Then**: Each pop returns the correct FIFO element; final `len()` is `0`

## Contract Verification Tests

### test_invariant_capacity_immutable
**Given**: A `ShardCommandQueue` after arbitrary sequence of enqueue/pop operations
**When**: `capacity()` is called and compared to initial `bounded_capacity()`
**Then**: They are equal (capacity never changes)

### test_invariant_len_bounded
**Given**: A `ShardCommandQueue` after arbitrary sequence of enqueue/pop operations
**When**: `len()` is compared to `0` and `capacity()`
**Then**: `0 <= len() <= capacity()`

### test_invariant_remaining_capacity_correct
**Given**: A `ShardCommandQueue` after arbitrary sequence of enqueue/pop operations
**When**: `remaining_capacity()` is compared to `capacity() - len()`
**Then**: They are equal

### test_invariant_is_full_equivalent
**Given**: A `ShardCommandQueue` after arbitrary sequence of enqueue/pop operations
**When**: `is_full()` is compared to `len() == capacity()`
**Then**: They are equal

## Given-When-Then Scenarios

### Scenario 1: Enqueue succeeds when space available
**Given**: `ShardCommandQueue::new(4)` produces a queue
**And**: no commands have been enqueued
**When**: `enqueue(SpawnActor { .. })` is called
**Then**:
- Return value is `Ok(())`
- `len()` is `1`
- `remaining_capacity()` is `3`
- `is_full()` is `false`

### Scenario 2: Enqueue fails with QueueFull when at capacity
**Given**: `ShardCommandQueue::new(2)` produces a queue
**And**: `enqueue(cmd_A)` and `enqueue(cmd_B)` have been called successfully
**When**: `enqueue(cmd_C)` is called
**Then**:
- Return value is `Err(RuntimeError::QueueFull)`
- `len()` is still `2`
- `remaining_capacity()` is still `0`
- `is_full()` is still `true`
- Queue contents unchanged: `[cmd_A, cmd_B]`

### Scenario 3: Pop returns commands in FIFO order
**Given**: `ShardCommandQueue::new(4)` produces a queue
**And**: `enqueue(cmd_1)`, `enqueue(cmd_2)`, `enqueue(cmd_3)` succeeded in that order
**When**: `pop()` is called 3 times
**Then**:
- First pop returns `Some(cmd_1)`
- Second pop returns `Some(cmd_2)`
- Third pop returns `Some(cmd_3)`
- Fourth pop returns `None`

### Scenario 4: Tick consumes at most one command
**Given**: A `Shard` with a `ShardCommandQueue` wrapper containing 3 commands
**When**: `tick()` is called once
**Then**:
- At most one command is dispatched
- Queue depth decreases by at most 1
- Remaining commands are still in FIFO order

### Scenario 5: QueueFull is deterministic
**Given**: A `ShardCommandQueue::new(2)` at capacity with `[cmd_A, cmd_B]`
**When**: `enqueue(cmd_C)` is called twice in immediate succession
**Then**:
- Both calls return `Err(RuntimeError::QueueFull)`
- No internal state mutation occurs on either failed call
