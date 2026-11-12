# Verification Layers: vb-c1s0

## Boundary Summary

| Layer | Scope |
|-------|-------|
| **TLA+** | Multi-shard routing, command processing order, run lifecycle state machine, timer wheel firing, liveness |
| **Verus** | Timer generation arithmetic, queue capacity invariants, budget exhaustion correctness, pure match functions |
| **Kani** | Bounded panic-freedom for tick paths, multi-shard routing, command processing |
| **Miri** | Unsafe Send+Sync on `BoundedActionCompletionQueue`, Mutex poisoning handling |
| **Loom** | Concurrent command queue ordering across shards |
| **Proptest** | Workflow primitive invariants (reduce, repeat, reentry, for_each, collect) |
| **Fuzz** | Journal event parsing, recovery reconstruction |
| **Integration** | BDD scenario execution via `cargo test` |
| **Gauntlet** | Moon v2 verification lanes for release gating |

---

## Layer Assignment

### INV-001: RunId Shard Consistency

| Layer | Tool | Command |
|-------|------|---------|
| **Primary** | `tla-plus` | `tlc -config specs/MultiShardRuntime.cfg specs/MultiShardRuntime.tla` |
| **Secondary** | `kani` | `cargo kani --package vb_runtime --harness tick_all` |
| **Secondary** | `integration` | `cargo test --package vb_runtime recovery_bdd_tests` |

### INV-002: Timer Generation Monotonicity

| Layer | Tool | Command |
|-------|------|---------|
| **Primary** | `verus` | `verus crates/vb_runtime/src/shard/timer_wheel.rs` |
| **Secondary** | `proptest` | `cargo test --package vb_runtime timer_wheel -- --test-threads=4` |

### INV-003: No Phantom Timer Delivery

| Layer | Tool | Command |
|-------|------|---------|
| **Primary** | `verus` | `verus crates/vb_runtime/src/shard/timer_wheel.rs` |
| **Secondary** | `kani` | `cargo kani --package vb_runtime --harness timer_entry_fired` |

### INV-004: Action Queue FIFO Ordering

| Layer | Tool | Command |
|-------|------|---------|
| **Primary** | `verus` | `verus crates/vb_runtime/src/action_queue.rs` |
| **Secondary** | `miri` | `MIRIFLAGS=-Zmiri-strict-provenance cargo miri test --package vb_runtime action_queue` |

### INV-005: Bounded Queue Capacity

| Layer | Tool | Command |
|-------|------|---------|
| **Primary** | `verus` | `verus crates/vb_runtime/src/action_queue.rs` |
| **Secondary** | `miri` | `MIRIFLAGS=-Zmiri-strict-provenance cargo miri test --package vb_runtime action_queue` |

### INV-006: Budget Exhaustion Safety

| Layer | Tool | Command |
|-------|------|---------|
| **Primary** | `verus` | `verus crates/vb_core/src/engine/run_loop.rs` |
| **Secondary** | `kani` | `cargo kani --package vb_core --harness run_until_blocked` |

### INV-007: Shard Command FIFO Processing

| Layer | Tool | Command |
|-------|------|---------|
| **Primary** | `tla-plus` | `tlc -config specs/ShardProcessing.cfg specs/ShardProcessing.tla` |
| **Secondary** | `loom` | `cargo loom --package vb_runtime --test shard_tick` |

### POST-002: Run Terminal State Uniqueness

| Layer | Tool | Command |
|-------|------|---------|
| **Primary** | `tla-plus` | `tlc -config specs/RunLifecycle.cfg specs/RunLifecycle.tla` |
| **Secondary** | `kani` | `cargo kani --package vb_runtime --harness terminal_state_invariants` |

### POST-003: Action Completion Routing

| Layer | Tool | Command |
|-------|------|---------|
| **Primary** | `tla-plus` | `tlc -config specs/ActionRouting.cfg specs/ActionRouting.tla` |
| **Secondary** | `integration` | `cargo test --package vb_runtime recovery_bdd_tests::action_completion` |

### POST-004: Timer Authority Handoff

| Layer | Tool | Command |
|-------|------|---------|
| **Primary** | `tla-plus` | `tlc -config specs/TimerWheel.cfg specs/TimerWheel.tla` |
| **Secondary** | `verus` | `verus crates/vb_runtime/src/shard/timer_wheel.rs` |

### POST-005: tick_all Progress

| Layer | Tool | Command |
|-------|------|---------|
| **Primary** | `tla-plus` | `tlc -config specs/ShardProcessing.cfg specs/ShardProcessing.tla` |
| **Secondary** | `kani` | `cargo kani --package vb_runtime --harness tick_all` |

### POST-006: Action Queue Backpressure

| Layer | Tool | Command |
|-------|------|---------|
| **Primary** | `verus` | `verus crates/vb_runtime/src/action_queue.rs` |
| **Secondary** | `integration` | `cargo test --package vb_runtime action_queue_backpressure` |

---

## Verus Scope

### Target: `vb_runtime::shard::timer_wheel`

```verus
// TimerWheel invariants
invariant [
    self.by_run.len() == self.by_deadline.iter().map(|(_, v)| v.len()).sum()
]

// Generation monotonicity
proof fn generation_monotonic(run: RunId, g1: u64, g2: u64)
    requires
        old(self).by_run.contains(run),
        old(self).by_run[run].generation == g1,
        new(self).by_run[run].generation == g2,
    ensures
        g2 == g1 + 1,
{
    // insert() increments generation by exactly 1
}

// Authority matching
spec fn matches_authority(entry: TimerEntry, pending: PendingTimer) -> bool {
    entry.generation == pending.generation
    && entry.deadline == pending.deadline
    && entry.kind == pending.kind
}
```

### Target: `vb_runtime::action_queue`

```verus
// Capacity invariant
invariant [
    self.inner.lock().items.len() <= self.capacity
]

// FIFO ordering preserved
proof fn fifo_preserved(old_queue: VecDeque<ActionTicket>, new_queue: VecDeque<ActionTicket>)
    requires
        new_queue == old_queue.push_back(ticket),
    ensures
        new_queue.len() == old_queue.len() + 1,
{
    // VecDeque push_back/pop_front preserves order
}
```

### Target: `vb_core::engine::run_loop`

```verus
// Budget exhaustion correctness
proof fn budget_exhaustion_correct(budget: StepBudget, steps: u16)
    requires
        budget.remaining() == steps,
    ensures
        budget.try_take().is_ok()
        ==> budget.remaining() == steps - 1,
        budget.try_take().is_err()
        ==> budget.remaining() == 0,
{
    // try_take() decrements or returns error at 0
}
```

---

## TLA+ Scope

### Module: `MultiShardRuntime`

| Element | Definition |
|---------|------------|
| Variables | `shards`, `shard_count`, `run_to_shard` |
| Init | `shard_count > 0`, `run_to_shard = [run \in Runs \|-> run.id % shard_count]` |
| Actions | `Submit`, `Resume`, `Cancel`, `CompleteAction`, `FailAction`, `AnswerAsk`, `TimerFired` |
| Safety | `NoDoubleRouting`, `RoutingDeterminism` |
| Evidence | `tlc -config specs/MultiShardRuntime.cfg specs/MultiShardRuntime.tla` |

### Module: `ShardProcessing`

| Element | Definition |
|---------|------------|
| Variables | `command_queues`, `processing` |
| Init | `command_queues = [i \in 1..shard_count \|-> <<>>]` |
| Actions | `TickAll`, `ProcessOneCommand` |
| Safety | `QueueFIFO`, `OneCommandPerTick` |
| Evidence | `tlc -config specs/ShardProcessing.cfg specs/ShardProcessing.tla` |

### Module: `RunLifecycle`

| Element | Definition |
|---------|------------|
| Variables | `runs`, `run_status` |
| Init | `run_status = [run \in Runs \|-> Nil]` |
| Actions | `Admit`, `Advance`, `SuspendAction`, `SuspendAsk`, `SuspendTimer`, `Resume`, `Finish`, `Fail`, `Cancel` |
| Safety | `TerminalUniqueness`, `NoCommandAfterTerminal` |
| Liveness | `EventuallyTerminal`, `EventuallyResumed` |
| Evidence | `tlc -config specs/RunLifecycle.cfg specs/RunLifecycle.tla` |

### Module: `TimerWheel`

| Element | Definition |
|---------|------------|
| Variables | `by_deadline`, `by_run` |
| Init | `by_deadline = {}`, `by_run = {}` |
| Actions | `Insert`, `Cancel`, `FireExpired` |
| Safety | `GenerationMonotonic`, `NoPhantomFire` |
| Evidence | `tlc -config specs/TimerWheel.cfg specs/TimerWheel.tla` |

---

## Loom Scope

### Target: `vb_runtime::runtime::tick_all` with concurrent shards

**Property**: Command queue ordering is preserved across concurrent `tick_all` calls on different shards.

```rust
// Loom model
#[test]
fn concurrent_tick_all_preserves_order() {
    loom::model(|| {
        let runtime = Runtime::new(NonZeroUsize::new(2).unwrap(), ShardConfig::default());
        let run = RunId::new(1);
        // Submit commands, tick, verify FIFO
    });
}
```

**Evidence**: `cargo loom --package vb_runtime tick_all_concurrent`

---

## Fuzz Scope

### Target: Journal event parsing and recovery reconstruction

**Property**: Arbitrary journal sequences do not cause panic or assertion failure during recovery.

**Evidence**: `cargo fuzz run journal_parse -- -runs=5000`

---

## Integration/BDD Scope

### BDD Scenario Files

| File | Scenario Count | Evidence Command |
|------|---------------|------------------|
| `crates/vb_cli/tests/cli_vb_m214_bdd_scenarios.rs` | 17+ | `cargo test --package vb_cli cli_vb_m214_bdd_scenarios` |
| `crates/vb_cli/tests/cli_verify_integration.rs` | 6 | `cargo test --package vb_cli cli_verify_integration` |
| `crates/vb_runtime/tests/recovery_bdd_tests.rs` | 20 (B-001 to B-020) | `cargo test --package vb_runtime recovery_bdd_tests` |
| `crates/vb_runtime/src/primitives/reentry_tests.rs` | 6 (GWT-RE-1 to GWT-RE-6) | `cargo test --package vb_runtime reentry` |
| `crates/workspace_tests/src/acceptance_catalog.rs` | 21 | `cargo test --package workspace_tests acceptance_catalog` |

---

## Waivers

| Clause | Owner | Reason | Expiry | Compensating Evidence |
|--------|-------|--------|--------|----------------------|
| Lean projection | Lewis | All critical Rust-local behavior is expressible in Verus | N/A | Verus proof obligations |
| Flux RS | Lewis | Verus is the standard for this codebase | N/A | Verus obligations |
| Crux-MIR | Lewis | Kani provides bounded checking; Miri for UB | N/A | Kani + Miri |
| External system integration proof | Lewis | Not machine-verifiable; manual QA only | Ongoing | Hands-on QA skill |
