# Proof Plan Review Input — vb-0253.1
**Bead**: vb-0253.1
**Workspace**: /tmp/vb-ws/vb-0253.1
**For**: proof-reviewer skill
**State**: 4 (Proof Planning)

---

## What This Bead Does

Adds `ShardCommandQueue` — a named domain wrapper around `crossbeam_queue::ArrayQueue<ShardCommand>` — to `vb_runtime/shard/types.rs`. The wrapper exposes a narrow, safe, non-blocking bounded queue API with domain terms (`enqueue`, `pop`, `tick`, `remaining_capacity`, `is_full`).

**No unsafe code. No concurrency changes. No reconfiguration of capacity.**

---

## Contract Clauses and Their Evidence

| Clause | What It Means | Evidence Type |
|--------|---------------|---------------|
| INV-001 | capacity fixed at construction | Verus (deferred) + TEST-CAPACITY-001 |
| INV-002 | 0 ≤ len ≤ capacity | PROPTEST-INV-002 + TEST-QUEUE-STATUS-001 |
| INV-003 | remaining = capacity - len | PROPTEST-INV-003 + TEST-QUEUE-STATUS-002 |
| INV-005 | pop returns FIFO oldest | Verus INV-005 (deferred) + queue tests |
| POST-001 | new() initializes with exact capacity | TEST-CAPACITY-001 + Verus POST-001 (deferred) |
| POST-002 | enqueue Ok iff push succeeds; QueueFull on fail; no block/alloc | TEST-QUEUEFULL-001 + PROPTEST-POST-002 + Verus POST-002 (deferred) |
| POST-003 | successful enqueue increments len by 1 | TEST-QUEUE-STATUS-001 + PROPTEST-INV-003 |
| POST-004 | failed enqueue leaves len/remaining/is_full unchanged | TEST-QUEUEFULL-002 |
| POST-005 | pop returns Some FIFO or None; never modifies capacity | TEST-QUEUEFULL-001 + Verus POST-005 (deferred) |
| POST-006 | pop decrements len by 1 | TEST-QUEUE-STATUS-001 + TEST-QUEUE-STATUS-002 |
| POST-007 | tick consumes at most one command | TLA-QUEUE-003 (deferred) + TEST-QUEUEFULL-001 |
| POST-008 | len/remaining/is_full/capacity consistent | TEST-QUEUE-STATUS-001 + TEST-QUEUE-STATUS-002 + Verus POST-008 (deferred) |
| ERR-001 | QueueFull is deterministic | PROPTEST-POST-002 + Verus ERR-001 (deferred) |
| ERR-002 | try_new InvalidConfiguration on 0 or >MAX | TEST-CAPACITY-001 + Verus ERR-002 (deferred) |

---

## Why verify-standard is Sufficient

This is a **delegation wrapper**. Every method is a direct, static call to `ArrayQueue`:

```
enqueue → ArrayQueue::push  (returns Result, no blocking, no alloc)
pop     → ArrayQueue::pop   (returns Option, FIFO)
len     → ArrayQueue::len
capacity→ stored field (set once at construction)
remaining_capacity → capacity - len
is_full  → len == capacity
bounded_capacity → const 65536
```

`crossbeam_queue::ArrayQueue` is already:
- Battle-tested (used in crossbeam itself)
- MPMC thread-safe
- Bounds-checked (panics on push_full, not on logic errors)

The wrapper adds **zero computational logic**. It renames and surfaces domain terms. The only behavior not already proven by `ArrayQueue` is the error taxonomy (`QueueFull` vs panic on overflow) — which is a matter of error translation, not correctness.

---

## Risk Assessment

| Risk | Level | Mitigation |
|------|-------|------------|
| Public API semver break | HIGH | `cargo semver-checks` gate at state 10 |
| Performance regression (hot enqueue path) | HIGH | `cargo asm` gate at state 10 vs baseline |
| Test regression | HIGH | All 5 existing test files run at state 7 gate |
| Wrong error variant mapping | MEDIUM | PROPTEST-POST-002 exhaustively tests Ok/Err mapping |
| Invariant violation across enqueue/pop | MEDIUM | PROPTEST-INV-002/003 stress 10k iterations |

---

## Reviewer Questions

1. Is `verify-standard` alone acceptable for a pure API delegation wrapper where the underlying data structure is already trusted?
2. Should the deferred Verus/TLA+ obligations block landing, or can they be tracked as `DEFERRED_GLOBAL` debt?
3. Is the assembly comparison (ASM-001) necessary, or is the zero-cost wrapper claim self-evident from direct delegation?
