# Contract Specification — vb-0253.1

## Context

- **Feature**: Wrap `crossbeam_queue::ArrayQueue<ShardCommand>` behind a domain-named `ShardCommandQueue` boundary in the `vb_runtime` crate.
- **Domain terms**: `ShardCommandQueue`, `ShardCommand`, `QueueFull`, `enqueue`, `pop`, `tick`, `capacity`, `remaining_capacity`.
- **Assumptions**:
  - `ArrayQueue<ShardCommand>` is already thread-safe (MPMC); wrapper does not change concurrency semantics.
  - `#![forbid(unsafe_code)]` applies; no unsafe needed in wrapper.
  - Capacity is set once at `ShardCommand::try_new(capacity)` and never changes.
  - All queue operations are non-blocking; `enqueue` never blocks or allocates on full.
- **Open questions**:
  - None: wrapper name, API surface, and behavioral contract fully specified in delivery-scope.jsonl.

---

## Preconditions

- **PRE-001**: `ShardCommandQueue::new(capacity: usize)` requires `capacity > 0` and `capacity <= MAX_COMMAND_QUEUE_CAPACITY` (65536). Calls `ShardCommand::try_new(capacity)` which returns `Err(RuntimeError)` if capacity is invalid.
- **PRE-002**: `enqueue(&self, cmd: ShardCommand)` has no caller-side preconditions beyond the receiver being alive (standard Rust borrow rules). The queue may or may not have room.
- **PRE-003**: `pop(&self)` has no caller-side preconditions. Returns `None` when the queue is empty.
- **PRE-004**: `tick(&self)` callers must hold exclusive access to the shard (enforced by `Shard`'s internal state, not by the wrapper). At most one command is consumed per call regardless of queue depth.

---

## Postconditions

- **POST-001**: `ShardCommandQueue::new(capacity)` returns `Ok(Self)` with the inner `ArrayQueue` initialized to exactly `capacity` slots. `capacity()` and `bounded_capacity()` always return that fixed value.
- **POST-002**: `enqueue(&self, cmd)` returns `Ok(())` **iff** the inner `ArrayQueue.push(cmd)` succeeds. Returns `Err(RuntimeError::QueueFull)` if the queue is at capacity. **Does not block. Does not allocate on full.**
- **POST-003**: After a successful `enqueue`, `len()` increases by exactly 1 and `remaining_capacity()` decreases by exactly 1.
- **POST-004**: After a failed `enqueue` (QueueFull), `len()`, `remaining_capacity()`, and `is_full()` are unchanged from immediately before the call.
- **POST-005**: `pop(&self)` returns `Some(cmd)` for the frontmost enqueued `ShardCommand` (FIFO) **iff** the queue is non-empty. Returns `None` if the queue is empty. Does not modify capacity.
- **POST-006**: After a successful `pop`, `len()` decreases by exactly 1 and `remaining_capacity()` increases by exactly 1.
- **POST-007**: `tick(&self)` calls `pop()` at most once. If a command was popped, it is dispatched to the appropriate handler. The wrapper itself does not implement tick logic; tick is a `Shard` method that uses the wrapper.
- **POST-008**: `len()`, `remaining_capacity()`, `is_full()`, and `capacity()` return values consistent with the inner `ArrayQueue` state at call time. `remaining_capacity()` = `saturating_sub(capacity(), len())`. `is_full()` = `len() == capacity()`.
- **POST-009**: `bounded_capacity(&self) -> usize` is a const/freeze function that returns the compile-time fixed capacity bound (65536).

---

## Invariants

- **INV-001**: `capacity()` is immutable from construction. `capacity() == initial_capacity` for the lifetime of the `ShardCommandQueue`.
- **INV-002**: `0 <= len() <= capacity()` always holds.
- **INV-003**: `len() + remaining_capacity() == capacity()` always holds (saturating arithmetic on remaining_capacity).
- **INV-004**: `is_full()` is equivalent to `len() == capacity()`.
- **INV-005**: `ArrayQueue`'s interior ring-buffer order is preserved by all wrapper reads (`pop` returns FIFO order of elements added by `enqueue`).
- **INV-006**: No `ShardCommandQueue` operation introduces interior mutability beyond what `ArrayQueue` already provides; the wrapper is `Sync + Send` because `ArrayQueue` is.

---

## Error Taxonomy

- **RuntimeError::QueueFull** — Returned by `enqueue` when the inner `ArrayQueue.push()` fails because the queue is at capacity. The error is **deterministic**: same queue state always produces `QueueFull` for the same `enqueue` call.
- **RuntimeError::InvalidConfiguration** — Returned by `ShardCommand::try_new(capacity)` when `capacity == 0` or `capacity > MAX_COMMAND_QUEUE_CAPACITY`. This is a **construction-time** error only.
- No other error variants are introduced by this bead.

---

## Contract Signatures

```rust
// ShardCommandQueue — public wrapper API
impl ShardCommandQueue {
    pub fn new(capacity: usize) -> Result<Self, RuntimeError>
    where Self: Sized;

    pub fn enqueue(&self, cmd: ShardCommand) -> RuntimeResult<()>;
    pub fn pop(&self) -> Option<ShardCommand>;
    pub fn len(&self) -> usize;
    pub fn capacity(&self) -> usize;
    pub fn remaining_capacity(&self) -> usize;
    pub fn is_full(&self) -> bool;
    pub fn bounded_capacity() -> usize;  // const / associated const
}

// ShardCommand — constructor
impl ShardCommand {
    pub fn try_new(capacity: usize) -> Result<Self, RuntimeError>;
}
```

---

## TLA+-Owned Clauses

See `tla-spec.md`. This bead describes a **data-structure wrapper** with fixed-capacity queue semantics. The key temporal property is that `tick` consumes **at most one** command per call, which is enforced by `Shard`'s `tick` implementation (not the wrapper itself). The wrapper provides the **`pop`** primitive; `Shard::tick` applies the at-most-one constraint on top of it. TLA+ is not the primary formal tool for this data-structure contract; Verus covers the pure Rust invariants.

---

## Verus-Owned Clauses

- **INV-001**: Proved via Verus `spec` function for `new` and a `proof fn` that capacity field is set once and never modified.
- **INV-002, INV-003**: Proved via Verus `invariant` on the wrapper's internal state, relating `len`, `capacity`, and `remaining_capacity`.
- **POST-002, POST-004**: Proved via Verus postcondition on `enqueue` mapping `ArrayQueue.push` result to `RuntimeResult`.
- **POST-005, POST-006**: Proved via Verus postcondition on `pop` describing FIFO semantics and state transitions.
- See `verification-layers.md` for full Verus obligation.

---

## Non-goals

- Changes to `ArrayQueue` internals or concurrency model.
- Introduction of unbounded queues.
- Changes to `ShardCommand` enum variants or dispatch logic.
- Runtime reconfiguration of queue capacity.
