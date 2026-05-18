# Test Plan: Section 39 Missing Benchmarks (B.4)

## Summary

- **Bead**: vb-qi37-4-bench (Section 39 benchmarks)
- **Problem**: Section 39 mandates 22 benchmark groups. 12 are MISSING.
- **Output**: `test-plan-benchmarks.md` — benchmark specifications for all 12 missing groups
- **Benchmark tool**: Criterion (per existing `benches/velvet_ballastics.rs` pattern)
- **Metadata format**: `profile=bench;tool=criterion-0.8;durability=mixed;mode=ir-and-generated;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected`

---

## Missing Benchmark Groups (12)

| # | Benchmark Group | Surface | Target Crate |
|---|-----------------|---------|--------------|
| 1 | `IR_traversal` | `CompiledWorkflow` node traversal | `vb_core` |
| 2 | `collect_page` | `collect_page` pagination | `vb_runtime` |
| 3 | `action_dispatch` | `ActionRegistry::dispatch` | `vb_runtime` |
| 4 | `memory_footprint` | run frame + value store allocation | `vb_core` |
| 5 | `cold_start` | new run from compiled workflow | `vb_core` |
| 6 | `pagination_cost` | `CollectStates` insert/find | `vb_runtime` |
| 7 | `action_queuing` | `ShardCommandQueue` enqueue/dequeue | `vb_runtime` |
| 8 | `timer_wheel_tick` | `TimerWheel::fire_expired` | `vb_runtime` |
| 9 | `snapshot_save` | `snapshot_from_state` + postcard encode | `vb_runtime` |
| 10 | `snapshot_restore` | frame hydration from snapshot | `vb_core` |
| 11 | `ArrayQueue` | `crossbeam_queue::ArrayQueue` push/pop | `vb_runtime` |
| 12 | `rtrb` | `rtrb::RingBuffer` push/pop | `vb_runtime` |

---

## 1. IR_traversal

**What it measures**: Cost of traversing the compiled workflow IR (nodes, expressions, accessors).

### Behavior: Depth-first traversal of all node kinds
Given: A `CompiledWorkflow` with mixed node kinds (SetConst, EvalExpr, BuildObject, BuildList, ChooseSlot, ForEach, Collect, Finish)
When: Depth-first traversal visits every node once
Then: Returns total node count with no panic

### Behavior: Breadth-first traversal of workflow graph
Given: A `CompiledWorkflow` with 1000+ nodes in a chain
When: BFS visits every node level-by-level
Then: Returns nodes in correct topological order

### Behavior: Expression program traversal
Given: A `CompiledWorkflow` with 10 expression programs
When: Traversal visits every `ExprProgram` and its ops
Then: All ops are visited with correct ordering

**Fixture strategy**:
- Small: `SMALL_WORKFLOW` (2 nodes)
- Medium: `save_chain_workflow(100)` (100 nodes)
- Large: `save_chain_workflow(1000)` (1000 nodes)

**Benchmark group**: `ir_traversal`

```rust
// ir_traversal_benches(c: &mut Criterion)
// Sub-benchmarks:
bench_ir_traversal_depth_first_small     — fixture=small_workflow;  surface=ir_traverse_df
bench_ir_traversal_depth_first_100_steps  — fixture=save_chain_100;  surface=ir_traverse_df
bench_ir_traversal_depth_first_1000_steps — fixture=save_chain_1000; surface=ir_traverse_df
bench_ir_traversal_bfs_1000_steps        — fixture=save_chain_1000;  surface=ir_traverse_bfs
bench_ir_traversal_expression_programs   — fixture=expr_workflow_10; surface=ir_traverse_exprs
```

**API boundary**: `CompiledWorkflow` (read-only), `WorkflowParts::nodes`, `CompiledNode::kind`

---

## 2. collect_page

**What it measures**: Per-page collection overhead for paginated list materialization.

### Behavior: First page collection
Given: A `CollectStates` table and a 100-item source list with page_size=50
When: `collect_page` is called for the first page
Then: Cursor advances to 50, current_page is populated, no error

### Behavior: Second page collection (continuation)
Given: An existing `CollectPaginationState` with cursor=50 for a 100-item list
When: `collect_page` is called for the second page
Then: Cursor advances to 100, remaining_count decrements correctly

### Behavior: Page exhausted (last page)
Given: An existing `CollectPaginationState` where cursor == item_count
When: `collect_page` is called
Then: Returns materialization complete signal, state is removed from table

### Behavior: Page limit exceeded
Given: An existing `CollectPaginationState` where cursor + page_size > limit
When: `collect_page` is called
Then: Returns only remaining items up to limit

**Fixture strategy**:
- 100-item list, page_size=50 (2 pages)
- 1000-item list, page_size=100 (10 pages)
- Time-limited: 100-item list, page_size=50, time_limit_ms=1

**Benchmark group**: `collect_page`

```rust
// collect_page_benches(c: &mut Criterion)
// Sub-benchmarks:
bench_collect_page_first_page_small      — fixture=list_100_page_50;  surface=collect_first_page
bench_collect_page_second_page           — fixture=list_100_page_50;  surface=collect_second_page
bench_collect_page_exhausted             — fixture=list_100_page_50;  surface=collect_exhausted
bench_collect_page_large_10_pages        — fixture=list_1000_page_100; surface=collect_10_pages
bench_collect_page_time_limit            — fixture=list_100_page_50_t1ms; surface=collect_time_limit
bench_collect_page_find_existing         — fixture=list_100_page_50;  surface=collect_find_state
```

**API boundary**: `CollectStates::upsert`, `CollectStates::find`, `collect_page` (from `primitives/collect.rs`)

---

## 3. action_dispatch

**What it measures**: Overhead of dispatching an action through `ActionRegistry`.

### Behavior: Successful dispatch to registered action
Given: An `ActionRegistry` with 10 registered actions
When: `dispatch` is called with a valid `ActionInput`
Then: Returns `ActionOutcome::Success` with correct result

### Behavior: Dispatch to unknown action
Given: An `ActionRegistry` with 10 registered actions
When: `dispatch` is called with an unregistered action ID
Then: Returns `ActionError::UnknownAction`

### Behavior: Dispatch with mismatched contract
Given: An `ActionRegistry` with registered action contracts
When: `dispatch` is called with input not matching contract
Then: Returns `ActionError::DispatchFailed`

### Behavior: Many-action registry lookup (100 actions)
Given: An `ActionRegistry` with 100 registered actions
When: Dispatch targets the last registered action
Then: Correct action is found without linear scan on successful path

**Fixture strategy**:
- 1 registered action
- 10 registered actions
- 100 registered actions (full registry)
- Unregistered action (error path)

**Benchmark group**: `action_dispatch`

```rust
// action_dispatch_benches(c: &mut Criterion)
// Sub-benchmarks:
bench_action_dispatch_single_registered   — fixture=1_action;   surface=action_dispatch
bench_action_dispatch_10_registered      — fixture=10_actions; surface=action_dispatch
bench_action_dispatch_100_registered     — fixture=100_actions; surface=action_dispatch
bench_action_dispatch_unknown_action     — fixture=10_actions; surface=action_dispatch_unknown
bench_action_dispatch_resolve_compile_time — fixture=10_actions; surface=resolve_compile_time
```

**API boundary**: `ActionRegistry::dispatch`, `ActionRegistry::resolve_compile_time`

---

## 4. memory_footprint

**What it measures**: Peak memory allocation for running a workflow through to completion.

### Behavior: Small workflow memory usage
Given: `SMALL_WORKFLOW` (2 steps: save + finish)
When: Run to completion
Then: Peak heap bytes allocated is within documented bound

### Behavior: Save chain 1000 memory usage
Given: `save_chain_workflow(1000)`
When: Run to completion
Then: Peak heap bytes scales linearly with slot count × 1000

### Behavior: ValueStore growth
Given: A `ValueStore` with 1000 slot writes
When: Each slot contains a 1KB value
Then: Allocated bytes reflect actual stored data + overhead

### Behavior: Frame pool reuse
Given: 100 runs of `SMALL_WORKFLOW` executed sequentially
When: Each run reuses a pooled frame
Then: Peak memory does NOT grow proportionally (pool amortizes allocation)

**Note**: This benchmark records memory via `memory_stats()` or `tracemalloc`. It is not a `Criterion` throughput benchmark — it reports peak RSS.

**Benchmark group**: `memory_footprint`

```rust
// memory_footprint_benches(c: &mut Criterion)
// Sub-benchmarks:
bench_memory_small_workflow_peak       — fixture=small_workflow;     surface=memory_peak_rss
bench_memory_save_chain_1000_peak      — fixture=save_chain_1000;  surface=memory_peak_rss
bench_memory_valuestore_growth         — fixture=1000_slot_writes;  surface=memory_growth
bench_memory_frame_pool_reuse          — fixture=small_workflow_x100; surface=memory_pool_reuse
```

**API boundary**: `RunFrame`, `ValueStore`, `FramePool`

---

## 5. cold_start

**What it measures**: Time to initialize a new run from a compiled workflow.

### Behavior: Cold start of small workflow
Given: A compiled `SMALL_WORKFLOW`
When: `new_run_frame` is called to create a new run
Then: Returns a valid `RunFrame` with initialized slots

### Behavior: Cold start of 1000-step workflow
Given: A compiled `save_chain_workflow(1000)`
When: `new_run_frame` is called
Then: Frame is created with 1001 nodes initialized

### Behavior: Cold start from YAML parse (full pipeline)
Given: YAML source text for `SMALL_WORKFLOW`
When: Full pipeline: parse → compile → new_run_frame
Then: Complete cold-start latency is measured

### Behavior: Concurrent cold starts (10 parallel runs)
Given: A compiled workflow
When: 10 `new_run_frame` calls are made concurrently
Then: All frames are created without contention errors

**Fixture strategy**:
- Small workflow (2 steps)
- Medium workflow (100 steps)
- Large workflow (1000 steps)
- From-YAML (full pipeline)

**Benchmark group**: `cold_start`

```rust
// cold_start_benches(c: &mut Criterion)
// Sub-benchmarks:
bench_cold_start_small              — fixture=small_workflow;     surface=new_run_frame
bench_cold_start_100_steps          — fixture=save_chain_100;    surface=new_run_frame
bench_cold_start_1000_steps        — fixture=save_chain_1000;   surface=new_run_frame
bench_cold_start_full_pipeline     — fixture=small_workflow_yaml; surface=parse_compile_frame
bench_cold_start_10_concurrent     — fixture=small_workflow;    surface=new_run_frame_concurrent
```

**API boundary**: `vb_core::new_run_frame`, `vb_compile::compile_workflow`, YAML parse pipeline

---

## 6. pagination_cost

**What it measures**: Cost of `CollectStates` table operations per page.

### Behavior: CollectStates insert (new pagination state)
Given: An empty `CollectStates` table
When: First page state is upserted
Then: State is stored and retrievable

### Behavior: CollectStates upsert (replace existing state)
Given: A `CollectStates` table with existing state for (RunId, SlotIdx)
When: Next-page state is upserted
Then: Previous page is recorded in lineage, new state replaces old

### Behavior: CollectStates find (existing entry)
Given: A `CollectStates` table with 100 active pagination states
When: `find` is called for an existing (RunId, SlotIdx, current_page)
Then: Correct state is returned in O(1) amortized

### Behavior: CollectStates find (missing entry)
Given: A `CollectStates` table with 100 active pagination states
When: `find` is called for a non-existent key
Then: Returns None without error

**Fixture strategy**:
- Empty table → 1 insert
- 1 entry → upsert (2nd page)
- 100 entries → find
- 100 entries → find missing

**Benchmark group**: `pagination_cost`

```rust
// pagination_cost_benches(c: &mut Criterion)
// Sub-benchmarks:
bench_pagination_insert_first          — fixture=empty_table;    surface=collect_states_insert
bench_pagination_upsert_second_page    — fixture=1_entry_table; surface=collect_states_upsert
bench_pagination_find_existing         — fixture=100_entry_table; surface=collect_states_find
bench_pagination_find_missing          — fixture=100_entry_table; surface=collect_states_find_missing
bench_pagination_lineage_tracking      — fixture=10_page_lineage; surface=collect_lineage
```

**API boundary**: `CollectStates::upsert`, `CollectStates::find`, `CollectPaginationState`

---

## 7. action_queuing

**What it measures**: `ShardCommandQueue` (backed by `ArrayQueue<ShardCommand>`) enqueue/dequeue throughput.

### Behavior: Enqueue on non-full queue
Given: A `ShardCommandQueue` with capacity 1024, currently empty
When: `enqueue(ShardCommand::Submit{..})` is called
Then: Returns Ok(()), queue len increases by 1

### Behavior: Dequeue on non-empty queue
Given: A `ShardCommandQueue` with 100 commands enqueued
When: `dequeue()` is called
Then: Returns Some(command), queue len decreases by 1, FIFO order preserved

### Behavior: Enqueue on full queue
Given: A `ShardCommandQueue` at capacity
When: `enqueue` is called
Then: Returns `RuntimeError::QueueFull`, queue unchanged

### Behavior: Concurrent enqueue/dequeue (single producer, single consumer)
Given: A `ShardCommandQueue` with capacity 1024
When: Producer enqueues while Consumer dequeues
Then: No data race, no lost commands, no double-fetch

**Fixture strategy**:
- Empty queue → enqueue
- 100 commands → dequeue (batch of 100)
- Full queue → enqueue (error path)
- Pre-filled queue → mixed enqueue/dequeue

**Benchmark group**: `action_queuing`

```rust
// action_queuing_benches(c: &mut Criterion)
// Sub-benchmarks:
bench_action_queue_enqueue            — fixture=queue_empty_1024;  surface=queue_enqueue
bench_action_queue_dequeue           — fixture=queue_100_items;   surface=queue_dequeue
bench_action_queue_full_enqueue_err   — fixture=queue_full;       surface=queue_enqueue_full
bench_action_queue_batch_100         — fixture=queue_empty_1024;  surface=queue_batch_100
bench_action_queue_len_is_full       — fixture=queue_1024_items;  surface=queue_len_is_full
```

**API boundary**: `ShardCommandQueue::enqueue`, `ShardCommandQueue::dequeue`, `ShardCommandQueue::is_full`

---

## 8. timer_wheel_tick

**What it measures**: `TimerWheel::fire_expired` overhead as timer count grows.

### Behavior: Fire expired with 0 timers
Given: An empty `TimerWheel`
When: `fire_expired(now)` is called
Then: Returns empty Vec, no allocation

### Behavior: Fire expired with 1 timer (expired)
Given: A `TimerWheel` with 1 timer whose deadline has passed
When: `fire_expired(now)` is called
Then: Returns the 1 timer entry, timer is removed from both indexes

### Behavior: Fire expired with 10 expired timers
Given: A `TimerWheel` with 10 timers all expired at the same instant
When: `fire_expired(now)` is called
Then: Returns 10 entries in deadline order

### Behavior: Fire expired with 100 timers (90 expired, 10 future)
Given: A `TimerWheel` with 100 timers (90 expired, 10 future)
When: `fire_expired(now)` is called
Then: Returns exactly 90 expired entries, 10 remain scheduled

### Behavior: Cancel existing timer
Given: A `TimerWheel` with 100 timers
When: `cancel(run_id)` is called for one existing timer
Then: Returns true, timer removed from both indexes

### Behavior: next_deadline when non-empty
Given: A `TimerWheel` with 10 timers
When: `next_deadline()` is called
Then: Returns Some(earliest deadline)

**Fixture strategy**:
- Empty wheel
- 1 expired timer
- 10 expired timers (same deadline)
- 100 timers (90 expired, 10 future)
- Cancel: 100 timers, cancel 1

**Benchmark group**: `timer_wheel_tick`

```rust
// timer_wheel_tick_benches(c: &mut Criterion)
// Sub-benchmarks:
bench_timer_wheel_fire_empty           — fixture=wheel_empty;         surface=fire_expired
bench_timer_wheel_fire_1_expired       — fixture=wheel_1_expired;    surface=fire_expired
bench_timer_wheel_fire_10_expired      — fixture=wheel_10_expired;   surface=fire_expired
bench_timer_wheel_fire_90_of_100       — fixture=wheel_100_mixed;    surface=fire_expired
bench_timer_wheel_cancel_1             — fixture=wheel_100;          surface=cancel
bench_timer_wheel_next_deadline         — fixture=wheel_10;           surface=next_deadline
bench_timer_wheel_insert_100            — fixture=wheel_empty;        surface=insert_100
```

**API boundary**: `TimerWheel::insert`, `TimerWheel::fire_expired`, `TimerWheel::cancel`, `TimerWheel::next_deadline`

---

## 9. snapshot_save

**What it measures**: Cost to serialize a run's state to a snapshot.

### Behavior: Snapshot of small run frame
Given: A `RunFrame` from `SMALL_WORKFLOW` after 1 step
When: `snapshot_from_state` is called
Then: Returns a `FrameStateSnapshot` with all slots and PC

### Behavior: Snapshot of 100-step run
Given: A `RunFrame` from `save_chain_workflow(100)` after 50 steps
When: `snapshot_from_state` is called
Then: Returns snapshot with correct PC=50, executed=50

### Behavior: Snapshot with large slot values (1KB each)
Given: A `RunFrame` with 10 slots each containing 1KB blob
When: Snapshot is created
Then: Serialized snapshot includes all 10KB of data

### Behavior: Snapshot serialization (postcard encode)
Given: A `FrameStateSnapshot`
When: `postcard::to_allocvec(&snapshot)` is called
Then: Returns serialized bytes, no allocation error

### Behavior: Snapshot with correlation ID
Given: A `RunFrame` mid-execution
When: `snapshot_from_state` is called with correlation=12345
Then: Snapshot preserves correlation in output

**Fixture strategy**:
- 1-step frame snapshot
- 50-step frame snapshot
- 1KB slot values × 10 slots
- With correlation ID

**Benchmark group**: `snapshot_save`

```rust
// snapshot_save_benches(c: &mut Criterion)
// Sub-benchmarks:
bench_snapshot_save_1_step        — fixture=frame_1_step;   surface=snapshot_from_state
bench_snapshot_save_50_steps      — fixture=frame_50_steps;  surface=snapshot_from_state
bench_snapshot_save_large_slots   — fixture=frame_10kb_slots; surface=snapshot_from_state
bench_snapshot_encode_postcard    — fixture=snapshot_small;   surface=postcard_encode
bench_snapshot_encode_100_slots   — fixture=snapshot_100_slots; surface=postcard_encode
```

**API boundary**: `vb_runtime::shard::helpers::snapshot_from_state`, `FrameStateSnapshot`, `postcard::to_allocvec`

---

## 10. snapshot_restore

**What it measures**: Cost to hydrate a `RunFrame` from a snapshot.

### Behavior: Restore from snapshot of 1-step run
Given: A `FrameStateSnapshot` captured after 1 step
When: Frame is hydrated from snapshot
Then: PC=1, executed=1, slot values restored

### Behavior: Restore from snapshot of 50-step run
Given: A `FrameStateSnapshot` captured after 50 steps
When: Frame is hydrated
Then: PC=50, executed=50, all slots match original state

### Behavior: Restore with large slot values (1KB each)
Given: A snapshot with 10 slots of 1KB each
When: Frame is hydrated
Then: All 10KB of slot data is correctly restored

### Behavior: Restore with CollectPaginationState
Given: A snapshot containing pagination state for a collect operation
When: Frame is hydrated
Then: Pagination state is included in restored `CollectStates`

### Behavior: Deserialization (postcard decode)
Given: Serialized snapshot bytes
When: `postcard::from_bytes::<FrameStateSnapshot>(&bytes)` is called
Then: Returns original snapshot with all fields intact

**Fixture strategy**:
- 1-step snapshot restore
- 50-step snapshot restore
- Large slot values
- Postcard decode overhead

**Benchmark group**: `snapshot_restore`

```rust
// snapshot_restore_benches(c: &mut Criterion)
// Sub-benchmarks:
bench_snapshot_restore_1_step      — fixture=snapshot_1_step;    surface=frame_restore
bench_snapshot_restore_50_steps    — fixture=snapshot_50_steps;   surface=frame_restore
bench_snapshot_restore_large_slots — fixture=snapshot_10kb_slots; surface=frame_restore
bench_snapshot_decode_postcard     — fixture=snapshot_encoded_50; surface=postcard_decode
bench_snapshot_restore_pagination  — fixture=snapshot_with_pagination; surface=restore_with_collect
```

**API boundary**: `FrameStateSnapshot`, frame hydration path, `postcard::from_bytes`

---

## 11. ArrayQueue

**What it measures**: Raw `crossbeam_queue::ArrayQueue` operations (the mandated backend per Section 50).

### Behavior: ArrayQueue push (non-full)
Given: An `ArrayQueue::<T>::new(capacity=1024)` that is empty
When: `push(item)` is called
Then: Returns Ok(()); len() increases by 1

### Behavior: ArrayQueue pop (non-empty)
Given: An `ArrayQueue::<T>` with 100 items
When: `pop()` is called
Then: Returns Some(item) in FIFO order; len() decreases by 1

### Behavior: ArrayQueue push on full
Given: An `ArrayQueue::<T>::new(capacity=1)` with 1 item
When: `push(second_item)` is called
Then: Returns Err(second_item) — item NOT lost, queue unchanged

### Behavior: ArrayQueue capacity boundary (1024 items)
Given: An `ArrayQueue::<T>::new(capacity=1024)`
When: 1024 items are pushed
Then: Queue is exactly full, push returns Err on 1025th attempt

### Behavior: ArrayQueue is_full and len consistency
Given: An `ArrayQueue::<T>::new(capacity=1024)` with 512 items
When: `is_full()` and `len()` are called
Then: `is_full()` returns false, `len()` returns 512

### Behavior: ArrayQueue SPSC correctness (1000 items)
Given: A queue with 1000 items
When: All 1000 items are popped in order
Then: FIFO order is preserved; final len() == 0

**Fixture strategy**:
- Empty queue → push
- 100 items → pop (batch)
- Full queue → push (error path)
- 1024 capacity boundary
- 1000-item FIFO verification

**Benchmark group**: `array_queue`

```rust
// array_queue_benches(c: &mut Criterion)
// Sub-benchmarks:
bench_array_queue_push_1            — fixture=aq_empty_1024;  surface=push
bench_array_queue_pop_1             — fixture=aq_100_items;    surface=pop
bench_array_queue_push_full_err     — fixture=aq_full_1;       surface=push_full_err
bench_array_queue_capacity_1024     — fixture=aq_empty_1024;   surface=push_capacity_boundary
bench_array_queue_is_full_len       — fixture=aq_512_items;    surface=is_full_len
bench_array_queue_fifo_1000         — fixture=aq_1000_items;   surface=fifo_1000
```

**API boundary**: `crossbeam_queue::ArrayQueue::<T>::push`, `pop`, `len`, `is_full`, `capacity`

**Note**: This benchmark MUST use `crossbeam_queue::ArrayQueue` directly (not a wrapper) to verify Section 50 compliance.

---

## 12. rtrb

**What it measures**: `rtrb::RingBuffer` (SPSC ring buffer for trace/action completion paths).

### Behavior: rtrb push (non-full)
Given: A `RingBuffer::<T, N>::new()` with default capacity
When: `push(item)` is called
Then: Returns Ok(()); available slots decrease

### Behavior: rtrb pop (non-empty)
Given: A `RingBuffer` with 100 items
When: `pop()` is called
Then: Returns Some(item) in FIFO order

### Behavior: rtrb push on full
Given: A `RingBuffer` at capacity
When: `push(item)` is called
Then: Returns Err(item) — item NOT lost, buffer unchanged

### Behavior: rtrb peek (read without consume)
Given: A `RingBuffer` with 100 items
When: `peek()` is called
Then: Returns Some(&item) at head without modifying buffer

### Behavior: rtrb is_full and is_empty
Given: A `RingBuffer` with 50 items in a 128-capacity buffer
When: `is_full()` and `is_empty()` are called
Then: `is_full()` returns false, `is_empty()` returns false

### Behavior: rtrb SPSC throughput (1000 items)
Given: A pre-filled `RingBuffer` with 1000 items
When: All items are popped
Then: FIFO order preserved, final `is_empty()` == true

**Fixture strategy**:
- Empty buffer → push
- 100 items → pop (batch)
- Full buffer → push (error path)
- Peek without consume
- 1000-item SPSC FIFO

**Benchmark group**: `rtrb`

```rust
// rtrb_benches(c: &mut Criterion)
// Sub-benchmarks:
bench_rtrb_push_1              — fixture=rtrb_empty;     surface=push
bench_rtrb_pop_1               — fixture=rtrb_100_items; surface=pop
bench_rtrb_push_full_err        — fixture=rtrb_full;      surface=push_full_err
bench_rtrb_peek                — fixture=rtrb_100_items; surface=peek
bench_rtrb_is_full_is_empty    — fixture=rtrb_50_items;  surface=is_full_is_empty
bench_rtrb_fifo_1000           — fixture=rtrb_1000_items; surface=fifo_1000
```

**API boundary**: `rtrb::RingBuffer::push`, `pop`, `peek`, `is_full`, `is_empty`, `len`

**Note**: Per master plan Section 50, `rtrb` is required for SPSC trace/action completion paths. This benchmark verifies that requirement.

---

## Cross-Cutting Concerns

### Fixture Infrastructure

Each benchmark group must reuse the existing fixture helpers from `benches/velvet_ballastics.rs`:
- `SMALL_WORKFLOW`, `CHOOSE_WORKFLOW`
- `save_chain_workflow(n)` — creates a chain of n `SetConst` nodes + Finish
- `finish_workflow()` — single Finish node
- `many_step_workflow(n)` — YAML string with n steps
- `expression_workflow()` — workflow with EvalExpr node

New helper functions needed for missing benchmarks:
```rust
// In benches/velvet_ballastics.rs
fn large_ir_workflow(node_count: u16) -> CompiledWorkflow
fn collect_source_list(item_count: usize, page_size: usize) -> Vec<SlotValue>
fn action_registry_with_n_actions(n: u16) -> ActionRegistry
fn timer_wheel_with_n_expired(n: u16, now: Instant) -> TimerWheel
fn frame_snapshot_after_n_steps(n: u16) -> FrameStateSnapshot
fn rtrb_ringbuffer_with_n_items(n: usize) -> RingBuffer<TraceEvent>
```

### Metadata String Format

All benchmarks MUST use the existing `metadata()` helper:
```rust
fn metadata(name: &str, fixture: &[u8], extra: &str) -> String
// Output format: "{name};{BENCH_METADATA};{extra};fixture_bytes={len};fixture_digest={blake3}"
```

`BENCH_METADATA` constant (already defined):
```rust
const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir-and-generated;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";
```

### Latency Budget Gates

All benchmarks MUST use `checked_iter` (already defined in `benches/velvet_ballastics.rs`):
- Respects `VB_BENCH_LATENCY_BUDGET_US` env var (default: 100_000 µs = 100ms)
- Reports budget utilization on success
- Asserts elapsed ≤ budget on each iteration
- This prevents micro-benchmarks from accidentally measuring unbounded work

### Benchmark Groups to Register

```rust
criterion_group!(
    benches,
    // Existing (already present):
    parse_yaml_benches,
    compile_and_validate_benches,
    expression_benches,
    slot_and_transition_benches,
    storage_and_ipc_benches,
    generated_benches,
    ir_vs_generated_benches,
    taint_scalar_expr_bench,
    taint_slot_loading_bench,
    taint_build_object_bench,
    taint_build_list_bench,
    taint_full_workflow_bench,
    submit_artifact_benches,
    budget_compute_benches,
    evidence_chain_benches,
    admission_gate_benches,
    capability_check_benches,
    // NEW — 12 missing benchmark groups:
    ir_traversal_benches,       // 1. IR_traversal
    collect_page_benches,       // 2. collect_page
    action_dispatch_benches,    // 3. action_dispatch
    memory_footprint_benches,   // 4. memory_footprint
    cold_start_benches,         // 5. cold_start
    pagination_cost_benches,    // 6. pagination_cost
    action_queuing_benches,     // 7. action_queuing
    timer_wheel_tick_benches,   // 8. timer_wheel_tick
    snapshot_save_benches,      // 9. snapshot_save
    snapshot_restore_benches,   // 10. snapshot_restore
    array_queue_benches,        // 11. ArrayQueue
    rtrb_benches                // 12. rtrb
);
criterion_main!(benches);
```

---

## Open Questions

1. **memory_footprint units**: Should this use `tracemalloc`, `memfd`, or just delta measurements? The existing `benches/` infrastructure uses Criterion which is for time measurements. Consider separating memory benchmarks into their own binary using `divan` or `iai` for accurate memory measurement.

2. **cold_start — concurrent runs**: The "10 concurrent cold starts" benchmark requires `std::thread::scope` or `crossbeam` to create true concurrency. Is the benchmark target single-threaded cold-start latency or multi-threaded throughput?

3. **collect_page fixture dependency**: The `collect_page` benchmark depends on `vb_runtime::primitives::collect::collect_page`. Need to confirm this is a public-free function or requires running through `execute.rs`. If it requires full runtime execution, the benchmark may need to run `run_until_blocked` through a collect workflow instead.

4. **snapshot_restore — frame hydration path**: The frame hydration from snapshot is not yet implemented as a standalone function. Need to confirm the actual API surface for restoring from `FrameStateSnapshot` — is it `RunFrame::try_from_snapshot()` or does it go through the shard?

5. **ArrayQueue capacity**: Per Section 50, `ArrayQueue` capacity is fixed at construction. The benchmark should verify that push on full returns the item (does not drop), matching the "no silent drop" requirement.

6. **rtrb ring buffer capacity**: The `rtrb` crate uses const generics for capacity. Benchmarks should test with `N=128` (trace ring) and `N=1024` (action completion ring) per the architecture.
