# Domain Model Review — vb-0253.1

## Reviewer: rust-contract (self-review; independent contract-verification-reviewer approval required before test planning)

## Domain Model: ShardCommandQueue

### Wrapped Type
- `crossbeam_queue::ArrayQueue<ShardCommand>` — bounded MPMC ring-buffer queue.

### Wrapper Justification
- The wrapper provides a **domain-named boundary** (`ShardCommandQueue`) separating direct `ArrayQueue` field access from call sites.
- The master document (velvet-ballistics-MASTER.md line 209, 225, 991) specifies `ArrayQueue` for bounded MPMC shard queues with fixed capacity and non-blocking admission.
- The wrapper does not change concurrency semantics, allocation behavior, or ordering guarantees of `ArrayQueue`.
- The wrapper does not introduce any new abstraction layers beyond the domain name.

### API Surface Review

| Method | Signature | Returns | Notes |
|--------|-----------|---------|-------|
| `new(capacity)` | `usize → Result<Self, RuntimeError>` | `Self` | Calls `ShardCommand::try_new(capacity)`, which validates `0 < capacity <= 65536` |
| `enqueue(&self, ShardCommand)` | `→ RuntimeResult<()>` | `Ok(())` or `Err(RuntimeError::QueueFull)` | Non-blocking, no allocation on full |
| `pop(&self)` | `→ Option<ShardCommand>` | `Some(cmd)` or `None` | FIFO order |
| `len(&self)` | `→ usize` | current depth | |
| `capacity(&self)` | `→ usize` | fixed at construction | |
| `remaining_capacity(&self)` | `→ usize` | `capacity() - len()` (saturating) | |
| `is_full(&self)` | `→ bool` | `len() == capacity()` | |
| `bounded_capacity()` | `→ usize` | `65536` (const) | |

### Error Taxonomy Review

| Error Variant | Semantic Meaning | Deterministic? |
|--------------|------------------|---------------|
| `RuntimeError::QueueFull` | `enqueue` failed because queue is at capacity | Yes — same queue state → same result |
| `RuntimeError::InvalidConfiguration` | `try_new(capacity)` failed: `capacity == 0` or `> MAX_COMMAND_QUEUE_CAPACITY` | Yes — pure function of input |

### Invariant Review

| Invariant | Holds? | Reasoning |
|-----------|--------|-----------|
| INV-001: capacity immutable | **Yes** | `ArrayQueue` capacity is set at construction and never exposed as mutable |
| INV-002: `0 ≤ len ≤ capacity` | **Yes** | `ArrayQueue` enforces upper bound; `len()` returns actual count |
| INV-003: `len + remaining = capacity` | **Yes** | By definition of both functions |
| INV-004: `is_full ≡ len == capacity` | **Yes** | Boolean equivalence |
| INV-005: FIFO order | **Yes** | `ArrayQueue::pop` is documented as FIFO; wrapper delegates directly |
| INV-006: `Sync + Send` | **Yes** | `ArrayQueue` is `Sync + Send`; wrapper has no interior mutability beyond it |

### Behavioral Contract Review

| Contract Clause | Verifiable Postcondition | Risk |
|-----------------|--------------------------|------|
| `enqueue` never blocks | Yes — `ArrayQueue::push` is non-blocking by design | Low |
| `enqueue` never allocates on full | Yes — `ArrayQueue::push` does not allocate | Low |
| `enqueue` returns `QueueFull` deterministically | Yes — postcondition maps push failure to error | Medium |
| `tick` consumes at most one command | **Partial** — enforced by `Shard::tick`, not `ShardCommandQueue::pop`; TLA+ model needed | Medium |
| Capacity fixed at construction | Yes — `new` consumes `capacity` as `usize` field | Low |

### Soundness Check

**No unsafe code introduced.** `#![forbid(unsafe_code)]` applies; wrapper uses only safe Rust idioms delegating to `ArrayQueue`'s safe API.

**No interior mutability beyond `ArrayQueue`.** Wrapper exposes shared `&self` access for all methods; `ArrayQueue` handles internal synchronization.

**No blocking or allocators on failure paths.** `enqueue` failure (`QueueFull`) returns immediately without allocating.

**No capability leaks.** `bounded_capacity()` is an associated const exposing the max capacity; no internal details leaked.

### Findings

1. **`tick` at-most-one constraint is on `Shard`, not `ShardCommandQueue`**: The wrapper provides `pop()` which returns the front element. The `Shard::tick()` method is responsible for calling `pop()` at most once per tick. This is correctly reflected in `tla-spec.md` (TLA-QUEUE-003) and `POST-007`.
2. **`chunk_025.rs` line 171 direct field access**: The test `shard.command_queue.len()` accesses the field directly. After wrapper introduction, this field access is replaced by the wrapper. The test needs updating to use `shard.command_queue.len()` through the public API (which will be `ShardCommandQueue::len()`). This is already flagged in delivery-scope.jsonl.
3. **No unbounded queue introduced**: Confirmed — `ArrayQueue` capacity is fixed at construction; no code path can change it.

### Verdict

**Domain model is sound.** `ShardCommandQueue` is a thin, well-justified domain wrapper over `ArrayQueue` with correct preconditions, postconditions, invariants, and error taxonomy. The `tick` at-most-one behavioral claim is correctly scoped to `Shard::tick` with TLA+ modeling.

**Pending independent review**: `contract-verification-reviewer` must write `contract-verification-review.md` with `STATUS: APPROVED` or `STATUS: REJECTED` before test planning may proceed.
