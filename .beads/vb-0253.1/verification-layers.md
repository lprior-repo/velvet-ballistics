# Verification Layers — vb-0253.1

## Boundary

- **Verus-owned kernel**: `ShardCommandQueue::new`, `enqueue`, `pop`, `len`, `capacity`, `remaining_capacity`, `is_full`. Pure Rust invariants and postconditions on these methods.
- **TLA+ temporal model**: Bounded-queue state machine, FIFO ordering, at-most-one-pop-per-tick behavioral claim on `Shard::tick`. See `tla-spec.md`.
- **Theorem projection**: None required.
- **Runtime shell**: `enqueue` failure maps `ArrayQueue.push` failure → `RuntimeError::QueueFull`. No blocking, no allocation on full.
- **External systems**: None.

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layer | Tertiary Layer |
|----------------|--------------|-----------------|----------------|
| INV-001 (capacity fixed) | `verus` | `cargo test` (unit) | — |
| INV-002 (0 ≤ len ≤ cap) | `verus` | `proptest` | — |
| INV-003 (len + remaining = cap) | `verus` | `cargo test` | — |
| INV-005 (FIFO order) | `verus` | `cargo test` | — |
| POST-001 (new constructor) | `verus` | `cargo test` | — |
| POST-002 (enqueue Ok/Err) | `verus` | `cargo test` (QueueFull) | — |
| POST-003/004 (len/remaining after enqueue) | `verus` | `proptest` | — |
| POST-005 (pop Option FIFO) | `verus` | `cargo test` | — |
| POST-006 (len/remaining after pop) | `verus` | `cargo test` | — |
| POST-007 (tick at-most-one) | `tla-plus` | `cargo test` | — |
| POST-008 (status methods consistency) | `verus` | `cargo test` | — |
| ERR-001 (QueueFull deterministic) | `verus` | `cargo test` | — |
| ERR-002 (InvalidConfiguration) | `verus` | `cargo test` | — |
| API-001 (ShardCommandQueue re-export) | `cargo test` (compile) | `clippy` | — |
| PERF-001 (zero-cost wrapper) | `cargo asm` (inspect) | `cargo bench` (micro) | — |
| Risk: public_api compatibility | `api-compat` | `cargo semver-checks` | — |

## Verus Scope

### Rust Target
- `crates/vb_runtime/src/shard/types.rs` — `ShardCommandQueue` struct and all public methods.

### Spec/Proof Functions
- `spec fn bounded_capacity() -> usize` — associated const returning 65536.
- `spec fn new(capacity: usize) -> ShardCommandQueue` — constructor spec with capacity freeze.
- `spec fn enqueue(self, cmd: ShardCommand) -> RuntimeResult<()>` — postconditions for Ok/Err.
- `spec fn pop(self) -> Option<ShardCommand>` — postconditions for Some/None and FIFO.
- `proof fn capacity_never_changes` — proof that `capacity()` is always the initial capacity.
- `proof fn len_bounds` — proof that `0 <= len() <= capacity()`.
- `proof fn remaining_capacity_correct` — proof that `remaining_capacity() = capacity() - len()`.
- `proof fn enqueue_no_alloc_on_full` — proof that `enqueue` failure does not mutate queue state.

### Invariants
- `capacity_immutable`: `capacity() == initial_capacity`
- `len_bounded`: `0 <= len() && len() <= capacity()`
- `remaining_correct`: `remaining_capacity() == capacity() - len()`
- `is_full_equivalent`: `is_full() == (len() == capacity())`

### Trusted Boundary
- `ShardCommand::try_new(capacity)` constructor (external validation of capacity range).

### Shell Exclusions
- I/O, async scheduling, storage, wall-clock time: none applicable.
- Only pure data-structure operations.

## TLA+ Scope

### Module/Model Path
- `specs/shard_command_queue.tla` — bounded queue state machine.
- `specs/shard_tick.tla` — at-most-one-per-tick behavioral model.

### Variables
- `queue_contents : Seq(CommandID)` — ordered sequence of enqueued commands.
- `CAPACITY : Nat` — fixed at model initialization.
- `tick_count : Nat` — number of ticks executed.

### Actions
- `Enqueue(cmd)` — appends `cmd` to `queue_contents` if not at capacity.
- `Pop` — removes `Tail(queue_contents)` if non-empty.
- `Tick` — calls `Pop` at most once.

### Safety Invariants
- `Len(queue_contents) ≤ CAPACITY`
- `FIFOOrder`: `Pop` always removes oldest unpopped element.

### Temporal Properties
- At-most-one-per-tick: `(tick_count' - tick_count) ≤ 1` per `Tick` action.

### Fairness/Deadlock Stance
- No fairness required (synchronous local data structure).
- No deadlock possible (no blocking operations).

### Refinement Boundary
- TLA+ `queue_contents` ↔ Rust `ArrayQueue<ShardCommand>` abstract sequence.
- TLA+ `CAPACITY` ↔ Rust `ShardCommandQueue::bounded_capacity()` = 65536.
- TLA+ `Pop` ↔ Rust `ShardCommandQueue::pop()`.
- TLA+ `Enqueue` ↔ Rust `ShardCommandQueue::enqueue(cmd) -> RuntimeResult<()>`.

### Evidence Command
```
tlc -config specs/shard_command_queue.cfg specs/shard_command_queue.tla
tlc -config specs/shard_tick.cfg specs/shard_tick.tla
```

## Performance Scope

### Zero-Cost Claim
- `ShardCommandQueue` is a zero-cost wrapper: direct field access to `ArrayQueue`, no additional runtime indirection on hot path.
- **Evidence**: `cargo asm --lib vb_runtime::shard::types::ShardCommandQueue::enqueue` and manual inspection of generated assembly to confirm no extra function pointer, no extra branch beyond `ArrayQueue::push`.

### Throughput Claim
- Queue operations are on the hot path for command ingestion.
- **Evidence**: micro-benchmark comparing `enqueue` + `pop` throughput before/after wrapper introduction. No regression beyond 2% is acceptable.

## Waivers

- **Theorem kernel**: No Lean/Aeneas/Hax required. See `lean-contract.md`.
- **Loom/Shuttle**: Not applicable — `ArrayQueue` is already proven-correct concurrent数据结构; wrapper does not introduce new concurrency.
- **Fuzzing**: Queue boundary conditions (full, empty, single element) are covered by exhaustive unit tests and proptest. Fuzzing added only if reviewer requires it.
