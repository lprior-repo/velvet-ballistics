# Proof Strategy: vb-c1s0 — BDD Orchestration Runtime Acceptance Scenarios

**Bead:** vb-c1s0  
**State:** Go-skill State 4 (Proof Planning)  
**Workspace:** /home/lewis/src/vb-c1s0-workspace  
**Source:** /home/lewis/src/velvet-ballistics  
**Generated:** 2026-05-19

---

## 1. Risk Classification

| Risk Tag | Evidence | Affected Obligations |
|----------|----------|---------------------|
| **Temporal / State-machine** | INV-001 (routing), INV-007 (FIFO tick), POST-002 (terminal states) | TLA-WF-001, TLA-WF-002, TLA-WF-003 |
| **Concurrency** | Multi-shard tick_all, action queue enqueue/dequeue | KANI-TICK-001, KANI-SHARD-001, LOOM-SHARD-001, LOOM-QUEUE-001 |
| **Rust-local invariant** | Timer generation monotonicity (INV-002), matches_authority (INV-003), queue FIFO (INV-004), capacity bound (INV-005), budget exhaustion (INV-006) | VERUS-INV-002 through VERUS-INV-006, VERUS-PRE-001, VERUS-PRE-004 |
| **Bounded state** | Action routing (POST-003), timer authority handoff (POST-004) | TLA-WF-004, TLA-WF-005 |
| **Unsafe / UB** | BoundedActionCompletionQueue uses Mutex; Send+Sync bounds | MIRI-QUEUE-001 |
| **Performance / backpressure** | 80% capacity warning, QueueFull error | KANI-QUEUE-001, VERUS-INV-005 |

---

## 2. Verifier Lane Strategy

### 2.1 TLA+ (5 obligations)

**Lane:** `tlc` model checker  
**Budget:** 3 state constraints per model; bounded depth  
**Trigger:** Temporal/workflow properties across multi-shard routing, FIFO command processing, terminal state uniqueness, timer authority handoff, action routing

| ID | Artifact | Model Bounds | Invariants |
|----|----------|--------------|------------|
| TLA-WF-001 | `specs/MultiShardRuntime.tla` | shard_count ≤ 4, MaxRuns ≤ 8 | RoutingDeterminism, NoDoubleRouting |
| TLA-WF-002 | `specs/ShardProcessing.tla` | MaxQueueDepth ≤ 3 | QueueFIFO, OneCommandPerTick |
| TLA-WF-003 | `specs/RunLifecycle.tla` | MaxSteps ≤ 5 | TerminalUniqueness, NoCommandAfterTerminal |
| TLA-WF-004 | `specs/TimerWheel.tla` | MaxTimers ≤ 4 | GenerationMonotonic, NoPhantomFire, DeadlineOrdering |
| TLA-WF-005 | `specs/ActionRouting.tla` | MaxPendingActions ≤ 8 | ActionRoutingCorrectness, TicketValidity |
| TLA-WF-006 | `specs/ShardProcessing.tla` (extended) | shard_count ≤ 4 | ShutdownCorrectness |

**Assumption:** TLA+ specs do not yet exist at state 4 — proof-writer must scaffold `specs/` directory with .tla and .cfg files before tlc can run.

**Waiver candidate:** TLA-WF-006 shares model with TLA-WF-002; if both pass on same model, consolidate to avoid redundant model-checking cost.

### 2.2 Verus (6 obligations)

**Lane:** `verus` with ghost/exec separation  
**Budget:** Pure functions only; no I/O, async, storage, journal, wall-clock time, or external callbacks  
**Trigger:** Rust-local arithmetic and data structure invariants

| ID | Target | Spec Fn | Proof Fn |
|----|--------|---------|----------|
| VERUS-INV-002 | `vb_runtime::shard::timer_wheel::TimerWheel::next_generation` | `next_generation_spec` | `proof_generation_monotonic` |
| VERUS-INV-003 | `vb_runtime::shard::timer_wheel::PendingTimer::matches_authority` | `matches_authority_spec` | `proof_matches_authority` |
| VERUS-INV-004 | `vb_runtime::action_queue::BoundedActionCompletionQueue` | `fifo_invariant_spec` | `proof_fifo_preserved` |
| VERUS-INV-005 | `vb_runtime::action_queue::BoundedActionCompletionQueue` | `capacity_bound_spec` | `proof_capacity_invariant` |
| VERUS-INV-006 | `vb_core::engine::run_loop::drive_deterministic` | `budget_exhaustion_spec` | `proof_budget_exhaustion_correct` |
| VERUS-PRE-001 | `vb_runtime::runtime::Runtime::new` | `requires_shard_count_positive` | `proof_new_precondition` |
| VERUS-PRE-004 | `vb_runtime::runtime::Runtime::timer_entry_fired` | `requires_timer_authority` | `proof_timer_entry_fired_precondition` |

**Assumption:** Rust source with Verus annotations does not yet exist at state 4 — proof-writer must add `verus!` blocks and spec/proof functions.

**Shell exclusions:** I/O, async scheduling, storage, wall-clock time, journal, external callbacks.

### 2.3 Kani (5 obligations)

**Lane:** `cargo kani --bounded`  
**Trigger:** Bounded panic-freedom on multi-shard routing, tick processing, timer insertion, queue enqueue, frame transitions

| ID | Harness | Scope |
|----|---------|-------|
| KANI-TICK-001 | `tick_all` | vb_runtime::runtime::Runtime::tick_all |
| KANI-SHARD-001 | `Shard_tick` | vb_runtime::shard::impl_parts::chunk_001::Shard::tick |
| KANI-TIMER-001 | `timer_insert` | vb_runtime::shard::timer_wheel::TimerWheel::insert |
| KANI-QUEUE-001 | `action_queue_enqueue` | vb_runtime::action_queue::BoundedActionCompletionQueue |
| KANI-FRAME-001 | `RunFrame_state` | vb_core::frame::RunFrame |

**Assumption:** Kani harnesses do not yet exist at state 4 — proof-writer must create `kani/` directory with harnesses.

**Note:** Use `kani::Arbitrary` for all core structures — no hardcoded dummy data.

### 2.4 Miri (1 obligation)

**Lane:** `MIRIFLAGS=-Zmiri-strict-provenance cargo miri test`  
**Trigger:** Unsafe code (BoundedActionCompletionQueue uses Mutex), Send+Sync bounds

| ID | Target | Evidence |
|----|--------|----------|
| MIRI-QUEUE-001 | `vb_runtime::action_queue` | No UB, no leaks, no panic in action_queue tests |

**Assumption:** Miri requires `cargo miri test` integration tests to exist. Check `crates/vb_runtime/tests/action_queue_tests.rs` or equivalent before running.

**DISCOVERY:** Miri cannot run without a compiled test binary. If no test exists, mark `blocked_tooling` with install instructions.

### 2.5 Loom (2 obligations)

**Lane:** `cargo loom`  
**Trigger:** Concurrent tick_all on different shards; concurrent enqueue/dequeue

| ID | Target | Evidence |
|----|--------|----------|
| LOOM-SHARD-001 | `vb_runtime::runtime::tick_all` with concurrent shards | No race conditions or ordering violations |
| LOOM-QUEUE-001 | `vb_runtime::action_queue` with concurrent enqueue/dequeue | No race conditions in concurrent access |

**Assumption:** Loom models do not exist at state 4 — proof-writer must create `loom/` directory with model files.

**DISCOVERY:** `cargo loom` requires loom package. Check availability via `cargoloom --version`; if unavailable, mark `blocked_tooling`.

### 2.6 Proptest (1 obligation)

**Lane:** `cargo test --package vb_runtime --lib primitives`  
**Trigger:** Workflow primitives with broad input space

| ID | Target | Iterations |
|----|--------|------------|
| PROPTEST-PRIM-001 | `vb_runtime::primitives` | 10,000 |

**Assumption:** Primitive implementations and proptest strategies exist in `crates/vb_runtime/src/primitives/`.

### 2.7 Integration (5 obligations)

**Lane:** `cargo test`  
**Trigger:** BDD acceptance scenarios, CLI operator workflows, recovery tests

| ID | Target | Expected |
|----|--------|----------|
| INTEGRATION-BDD-001 | `crates/vb_runtime/tests/recovery_bdd_tests.rs` | 20 recovery BDD tests pass (B-001 to B-020) |
| INTEGRATION-CLI-001 | `crates/vb_cli/tests/cli_vb_m214_bdd_scenarios.rs` | 17+ CLI BDD scenarios pass |
| INTEGRATION-CLI-002 | `crates/vb_cli/tests/cli_verify_integration.rs` | 6 verify BDD scenarios pass |
| INTEGRATION-CATALOG-001 | `crates/workspace_tests/src/acceptance_catalog.rs` | 21 catalog scenarios validate |

### 2.8 Gauntlet Gates (2 obligations)

**Lane:** `moon run :verify-proof` / `moon run :verify-all`  
**Trigger:** Release-critical full verification pass

| ID | Target | Evidence |
|----|--------|----------|
| GATE-PROOF-001 | `moon run :verify-proof` | All proof obligations PASS or WAIVED |
| GATE-ALL-001 | `moon run :verify-all` | Full gauntlet: proof + deep + standard |

---

## 3. Assumptions

| ID | Assumption | Source |
|----|------------|--------|
| ASM-001 | All shards operate single-threadedly; no intra-shard locking required | contract.md |
| ASM-002 | RunId → Shard routing via `run_id.get() % shard_count` is deterministic and consistent | contract.md |
| ASM-003 | Journal events provide the canonical replay evidence chain | contract.md |
| ASM-004 | Timer wheel generation arithmetic is bounded by u64::MAX | contract.md |
| ASM-005 | BoundedActionCompletionQueue capacity is fixed at construction and never changes | contract.md |
| ASM-006 | Every action ticket enqueued corresponds to an AwaitingAction signal from the engine | contract.md |
| ASM-007 | `drive_deterministic` loop exits only on Continue, non-Continue EngineSignal, or budget exhaustion | contract.md |

---

## 4. Budget Constraints

| Verifier | Time Budget | Notes |
|----------|-------------|-------|
| TLC (each model) | 5 min per model | Bounded state constraints keep state space tractable |
| Verus | 10 min per fn | Pure functions; no I/O |
| Kani | 15 min per harness | Bounded mode; honest unwind settings |
| Miri | 5 min | Short test runs only |
| Loom | 10 min per model | Schedule exploration with boundedconcurrency |
| Proptest | 5 min | 10k iterations with 4 threads |

---

## 5. Waiver Candidates

| Obligation | Reason | Compensating Evidence |
|------------|--------|----------------------|
| TLA-WF-006 | Shares model (ShardProcessing) with TLA-WF-002; redundant if both pass | If TLA-WF-002 passes with ShutdownCorrectness invariant added, TLA-WF-006 waived |
| LOOM-SHARD-001 | KANI-TICK-001 provides bounded panic-freedom; Loom is additional schedule exploration | KANI-TICK-001 evidence if loom blocked |
| PROPTEST-PRIM-001 | Low risk tag; primitives have limited state space | BDD integration tests cover primitives |

---

## 6. Open Questions (DISCOVERY_BLOCKED)

| ID | Question | Blocking |
|----|----------|----------|
| OQ-001 | Whether vb-c1s0 defines NEW BDD scenarios or validates EXISTING catalog scenarios (BDD-KYYF-001 through BDD-NJJU-004, VB-BDD-CATALOG-001 through 010) | INTEGRATION-CATALOG-001 scope |
| OQ-002 | Whether additional scenario scheduling beyond ActionScheduled/WaitScheduled/AskScheduled exists for compound workflows | TLA-WF-005 model scope |
| OQ-003 | Whether TLA+ specs already exist in /home/lewis/src/velvet-ballistics/specs/ | TLA+ obligation creation |
| OQ-004 | Whether Verus annotations already exist in Rust source | Verus obligation creation |
| OQ-005 | Whether loom models already exist | Loom obligation creation |

---

## 7. Artifact Targets

| Artifact | Owner | Rerun From |
|----------|-------|------------|
| `specs/MultiShardRuntime.tla` + `.cfg` | proof-writer | state 5 |
| `specs/ShardProcessing.tla` + `.cfg` | proof-writer | state 5 |
| `specs/RunLifecycle.tla` + `.cfg` | proof-writer | state 5 |
| `specs/TimerWheel.tla` + `.cfg` | proof-writer | state 5 |
| `specs/ActionRouting.tla` + `.cfg` | proof-writer | state 5 |
| `kani/harnesses/*.rs` | proof-writer | state 5 |
| `loom/models/*.rs` | proof-writer | state 5 |
| `verification/verus/*.rs` | proof-writer | state 5 |
| `proof-obligations.planned.jsonl` | proof-planner | state 4 |

---

*Generated by proof-planner skill. Status: planned. All obligations require proof-writer to create artifacts before execution.*
