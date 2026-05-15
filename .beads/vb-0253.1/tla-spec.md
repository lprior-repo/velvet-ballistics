# TLA+ Temporal Model Plan — vb-0253.1

## Boundary

- **Temporal/workflow behavior owned by TLA+**:
  - `ShardCommandQueue` enqueue/pop state machine: init with fixed capacity, enqueue action, pop action.
  - FIFO ordering invariant: popped element is the oldest successfully enqueued element not yet popped.
  - Bounded capacity invariant: `0 ≤ queue_depth ≤ capacity` at all states.
  - At-most-one-pop-per-tick: the `tick` method on `Shard` (which calls `ShardCommandQueue::pop`) is the TLA+ action that represents "at most one command consumed per tick call". This is a behavioral contract on `Shard::tick`, not the wrapper itself.
- **Rust/core behavior excluded from TLA+ and handled by Verus/tests**:
  - `ShardCommandQueue::new` constructor correctness and capacity freeze.
  - `enqueue` postconditions (Ok/Err mapping to `ArrayQueue.push` result).
  - `pop` postconditions (Option<ShardCommand>, FIFO order, state update).
  - `len`, `remaining_capacity`, `is_full`, `capacity` consistency proofs.
  - No allocation on full (`enqueue` failure path).
- **External systems abstracted**: None — this is an in-process data structure.
- **Non-applicability rationale**: The primary contract of `ShardCommandQueue` is a **local data-structure specification**, not a distributed protocol, workflow, or multi-agent coordination problem. TLA+ model-checking adds limited value for the FIFO bounded-queue invariants, which are better covered by Verus pure proofs and unit tests. TLA+ is applicable for the **at-most-one-per-tick** behavioral claim, which is the only temporal/behavioral property that spans multiple operations.

## TLA+-Owned Clauses

### TLA-QUEUE-001: Bounded Capacity Invariant
- **Contract clause**: INV-002
- **TLA+ module**: `ShardCommandQueue` (specs/shard_command_queue.tla)
- **Claim**: At all states, `queue_depth ≤ CAPACITY` where `CAPACITY` is a constant set at model initialization.
- **Variables**: `queue_depth : [0, CAPACITY]`, `CAPACITY : Nat`
- **Init**: `queue_depth = 0`
- **Actions**: `Enqueue` (increments `queue_depth` if `queue_depth < CAPACITY`), `Pop` (decrements `queue_depth` if `queue_depth > 0`)
- **Safety invariant**: `queue_depth ≤ CAPACITY`
- **Evidence**: `tlc -config specs/shard_command_queue.cfg specs/shard_command_queue.tla`

### TLA-QUEUE-002: FIFO Ordering
- **Contract clause**: INV-005
- **TLA+ module**: `ShardCommandQueue`
- **Claim**: `Pop` returns the element oldest in the queue (first-enqueued not-yet-popped). Modeled by explicit queue content variable (a sequence).
- **Variables**: `queue_contents : Seq(CommandID)`, `CAPACITY : Nat`
- **Init**: `queue_contents = <<>>`
- **Enqueue**: `queue_contents' = Append(queue_contents, cmd_id)` if `Len(queue_contents) < CAPACITY`
- **Pop**: `queue_contents' = Tail(queue_contents)` if `Len(queue_contents) > 0`
- **Safety invariant**: `Len(queue_contents) ≤ CAPACITY`
- **Evidence**: `tlc -config specs/shard_command_queue.cfg specs/shard_command_queue.tla`

### TLA-QUEUE-003: At-Most-One-Pop-Per-Tick
- **Contract clause**: POST-007 (tick at-most-one)
- **TLA+ module**: `ShardTick` (specs/shard_tick.tla)
- **Claim**: Each `Tick` action pops at most one element from the queue, regardless of queue depth.
- **Variables**: `queue_contents : Seq(CommandID)`, `tick_count : Nat`
- **Actions**: `Enqueue(cmd_id)` (append), `Tick` (pop exactly 0 or 1 element)
- **Safety invariant**: `tick_count` increments by at most 1 per `Tick` action; `queue_contents` decreases by at most 1 per `Tick`.
- **Temporal property**: `∀i : Int → (Tick(i) ⇒ Δqueue_contents(i) ∈ {-1, 0})`
- **Evidence**: `tlc -config specs/shard_tick.cfg specs/shard_tick.tla`

## Model Shape

### specs/shard_command_queue.tla
```
---- MODULE shard_command_queue ----
CONSTANT CAPACITY
VARIABLE queue_contents

CommandID == Nat
NullCmd == 0

Init == queue_contents = <<>>
Enqueue(cmd) == IF Len(queue_contents) < CAPACITY
                 THEN queue_contents' = Append(queue_contents, cmd)
                 ELSE UNCHANGED queue_contents
Pop == IF Len(queue_contents) > 0
        THEN queue_contents' = Tail(queue_contents)
        ELSE UNCHANGED queue_contents

TypeInvariant == queue_contents \in Seq(CommandID) /\ Len(queue_contents) ≤ CAPACITY

Invariants:
  TypeInvariant

====================================
```

### specs/shard_tick.tla
```
---- MODULE shard_tick ----
CONSTANT CAPACITY
VARIABLE queue_contents, tick_count

Init == queue_contents = <<>> /\ tick_count = 0
Enqueue(cmd) == IF Len(queue_contents) < CAPACITY
                 THEN /\ queue_contents' = Append(queue_contents, cmd)
                      /\ tick_count' = tick_count
                 ELSE /\ UNCHANGED queue_contents
                      /\ tick_count' = tick_count
PopOne == IF Len(queue_contents) > 0
           THEN /\ queue_contents' = Tail(queue_contents)
                /\ tick_count' = tick_count + 1
           ELSE /\ UNCHANGED queue_contents
                /\ tick_count' = tick_count

AtMostOnePerTick == (tick_count' - tick_count) ≤ 1
QueueDepthBounded == Len(queue_contents) ≤ CAPACITY

====================================
```

## Properties

### Safety Invariants
- `TypeInvariant` / `QueueDepthBounded`: queue never exceeds capacity.
- `FIFOOrder`: Pop always removes the oldest unpopped element.

### Liveness/Eventuality
- None required for this data-structure contract. Queue operations are synchronous and caller-driven.

### Fairness/Deadlock Stance
- No fairness requirements: this is a synchronous local data structure.
- No deadlock concerns.

### Refinement to Rust/runtime behavior
- TLA+ `queue_contents` models the abstract sequence of `ShardCommand`s in the `ArrayQueue`.
- TLA+ `CAPACITY` refines to `ShardCommandQueue::bounded_capacity()` (65536).
- TLA+ `Pop` refines to `ShardCommandQueue::pop()` → `Option<ShardCommand>`.
- TLA+ `Enqueue` refines to `ShardCommandQueue::enqueue(cmd)` → `RuntimeResult<()>` (QueueFull maps to `enqueue` leaving `queue_contents` unchanged).

## Evidence Command
```
tlc -config specs/shard_command_queue.cfg specs/shard_command_queue.tla
tlc -config specs/shard_tick.cfg specs/shard_tick.tla
```

## Waivers

- **TLA-QUEUE-001, TLA-QUEUE-002**: Low-value for local data structure; Verus proofs and unit tests cover these properties more directly and with less modeling overhead. Waived unless reviewer requires TLA+ model.
- **TLA-QUEUE-003**: Retained as TLA+ obligation because `tick at-most-one` is a behavioral/ordinal property that benefits from explicit state-machine modeling.
