# Theorem Kernel Projection: vb-c1s0

## Boundary

### TLA+-Owned Temporal Model
- Multi-shard command routing determinism
- Per-shard FIFO command processing
- Run lifecycle state transitions
- Timer wheel firing semantics (deadline ordering, generation matching)
- Action completion routing and run resumption
- Graceful shutdown sequence

**Artifacts**: `tla-spec.md` → TLA+ modules in `specs/MultiShardRuntime.tla`, `specs/ShardProcessing.tla`, `specs/TimerWheel.tla`, `specs/RunLifecycle.tla`

### Verus-Owned Rust Core
All Rust-local pure deterministic critical behavior is expressible in Verus:

| Clause | Rust Target | Verus Surface |
|--------|-------------|---------------|
| INV-002 | `vb_runtime::shard::timer_wheel::TimerWheel::next_generation` | `spec fn` + `proof fn` |
| INV-003 | `vb_runtime::shard::timer_wheel::TimerWheel::matches_authority` | `spec fn` pure match |
| INV-004 | `vb_runtime::action_queue::BoundedActionCompletionQueue` | `invariants` on `enqueue`/`dequeue` |
| INV-005 | `vb_runtime::action_queue::BoundedActionCompletionQueue` | `invariant` capacity bound |
| INV-006 | `vb_core::engine::run_loop::drive_deterministic` | `proof fn` budget exhaustion |
| PRE-001 | `vb_runtime::Runtime::new` | `requires` clause |
| PRE-004 | `vb_runtime::Runtime::timer_entry_fired` | `requires` generation match |

### Theorem-Owned Kernel
**None at this time.**

Rationale:
1. Timer generation arithmetic is bounded by `u64::MAX` — expressible in Verus
2. Queue capacity is a simple `usize` bound — expressible in Verus
3. Budget exhaustion is deterministic subtraction — expressible in Verus
4. No algebraic structure extraction is needed beyond what Verus can express

If future requirements expose:
- Algebraic proof of timer wheel O(log n) bounds under refinement
- Protocol lattice proofs for concurrent shard communication
- Arithmetic bound theorems requiring Coq/Lean extraction

Then a Lean kernel projection will be added.

### Rust/Runtime Shell (Excluded from Formal Proof)
- `Runtime::submit_direct` routing via `RunId % shard_count`
- `ShardCommand` queue processing (TLA+ owns this)
- `SharedRuntimeJournal` event emission
- Wall-clock `Instant` source
- External callback interfaces (`complete_action_with_output`, `answer_ask`)

### External Systems (Excluded)
- CLI user interaction
- Storage backend
- Network transport

---

## Theorem-Owned Clauses

**None.** Verus is sufficient for all Rust-local pure/core proof obligations.

---

## Lean Theorem Obligations

**None.** No Lean projection required.

---

## Verus Obligations (Summary)

| ID | Clause | Rust Target | Verus Surface |
|----|--------|-------------|----------------|
| VERUS-INV-002 | INV-002 | `TimerWheel::next_generation` | `spec fn next_generation_spec` + `proof fn generation_monotonic` |
| VERUS-INV-003 | INV-003 | `TimerWheel::matches_authority` | `spec fn` pure match validation |
| VERUS-INV-004 | INV-004 | `BoundedActionCompletionQueue` | `invariant` FIFO ordering |
| VERUS-INV-005 | INV-005 | `BoundedActionCompletionQueue` | `invariant` capacity bound |
| VERUS-INV-006 | INV-006 | `drive_deterministic` | `proof fn budget_exhaustion_correct` |
| VERUS-PRE-001 | PRE-001 | `Runtime::new` | `requires` shard_count > 0 |
| VERUS-PRE-004 | PRE-004 | `Runtime::timer_entry_fired` | `requires` generation match |

---

## Waivers

| Clause | Owner | Reason | Expiry | Compensating Evidence |
|--------|-------|--------|--------|----------------------|
| Lean projection | Lewis | All critical Rust-local behavior is expressible in Verus; no algebraic kernel extraction needed | N/A | Verus proof obligations + Kani bounded checking + Fowler tests |

---

## Open Questions

- **DISCOVERY_BLOCKED**: Whether a future Lean kernel is needed for timer wheel O(log n) amortized bound proof under adversarial scheduling
- **DISCOVERY_BLOCKED**: Whether protocol lattice proofs for multi-shard coordination are in scope for vb-c1s0
