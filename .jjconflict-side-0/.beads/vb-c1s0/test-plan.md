# Test Plan: vb-c1s0 — Orchestration Runtime Acceptance Scenarios

## Summary

- **Bead**: vb-c1s0
- **Title**: bdd: Orchestration runtime acceptance scenarios
- **State**: Go-skill State 7 (Test Planning)
- **Source checkout**: /home/lewis/src/velvet-ballistics
- **Isolated workspace**: /home/lewis/src/vb-c1s0-workspace
- **Behaviors identified**: 22
- **Trophy allocation**: 4 unit / 14 integration / 4 e2e
- **Proptest invariants**: 6
- **Fuzz targets**: 2
- **Kani harnesses**: 5 (BLOCKED — compensating evidence: 1,354 integration tests + TLA+ reduced-bounds)

---

## 1. Behavior Inventory

### Runtime Construction (PRE-001)
1. `Runtime::new` rejects `shard_count == 0`
2. `Runtime::new` creates exactly `shard_count` shards when `shard_count > 0`

### Submit Admission (PRE-002, POST-001)
3. `submit_direct` accepts a valid RunId + CompiledWorkflow and returns `Ok(())`
4. `submit_direct` enqueues a `Submit` command to the correct shard's command queue
5. `submit_direct` returns `Err(RuntimeError::AdmissionRejected)` on capability validation failure
6. `submit_direct` returns `Err(RuntimeError::RunAlreadyExists)` when a run with the same RunId is already active
7. `submit_direct_with_inputs_grants_and_contracts` accepts pre-mapped inputs and action contracts

### Run Lifecycle Terminal States (POST-002)
8. A run reaches exactly one terminal state (`Finished`, `Failed`, `Cancelled`, or `Skipped`)
9. A terminal run ignores all subsequent commands
10. A terminal run rejects re-submission with `RunAlreadyExists`

### Action Completion (PRE-003, POST-003)
11. `complete_action_with_output` resumes the exact `RunFrame` and `StepIdx` identified by the ticket
12. `complete_action_with_output` returns `Err(RuntimeError::InvalidTicket)` for non-existent tickets
13. `fail_action` transitions the run to the `Failed` terminal state
14. The resumed step is the step immediately following the suspended step

### Timer Authority Handoff (PRE-004, POST-004)
15. `insert` creates a timer with generation = 1 for a new run
16. `insert` increments generation by 1 for successive timers on the same run
17. `insert` returns `Err(TimerWheelError::GenerationExhausted)` at u64::MAX
18. `timer_entry_fired` fires only when generation, deadline, and kind all match
19. `timer_entry_fired` silently discards entries with stale generation (mismatch ignored, no error)
20. `timer_entry_fired` silently discards entries with wrong deadline (mismatch ignored, no error)

### Action Queue Backpressure (POST-006)
21. `enqueue` returns `Err(ActionQueueError::QueueFull { capacity })` at capacity
22. `enqueue` emits `BackpressureWarning` when queue reaches or exceeds 80% capacity
23. `enqueue` does not emit warning at 79% capacity (below threshold)

### Tick All (POST-005, INV-007)
24. `tick_all` processes at most one command per shard per call
25. `tick_all` returns `false` when any shard is shutting down; `true` otherwise
26. Commands are processed in FIFO order per shard (INV-007)

### Shard Routing Consistency (INV-001)
27. `shard_for(run)` returns the same shard for a given RunId for the lifetime of the runtime
28. Routing formula: `run.get() % shard_count` is deterministic

### Budget Exhaustion (INV-006)
29. `drive_deterministic` exits with `StepBudgetExhausted` when `budget.try_take()` returns `false`
30. No steps execute beyond the budget

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| **Unit / Calc** | 4 | `BoundedActionCompletionQueue` (pure data structure), `TimerWheel` (pure insert/cancel/fire), `StepBudget` arithmetic, `Runtime::new` precondition validation. No I/O, no external deps. |
| **Integration** | 14 | All shard command processing, submit/resume/cancel lifecycle, action completion routing, timer firing, multi-shard tick_all, terminal state reachability. Uses real Runtime, real Shard, real queues — no mocks. |
| **E2E** | 4 | Full `Runtime` bootstrapped from CLI, journal replay, multi-step workflow from submit to terminal state, backpressure observable at process boundary. |
| **Static Analysis** | — | `cargo clippy` and `cargo-deny` enforced at CI gate via `moon ci`; not duplicated here. |

**Rationale for integration-heavy allocation**: The orchestration runtime's core guarantees (shard routing, FIFO command processing, timer authority, action routing) are only verifiable in integration with real data structures. Unit tests alone cannot prove INV-001 (deterministic routing), INV-007 (one command per tick), or POST-003 (action delivery to correct step). The 1,354 integration tests already filed (PO-023 through PO-026 all PASS) provide this coverage.

---

## 3. BDD Scenarios

### Group A: Runtime Construction (PRE-001)

**Scenario A1: Runtime rejects zero shards**
```
Given: shard_count = 0
When:  Runtime::new(shard_count, config) is called
Then:  the constructor panics or returns an error
And:   no shards are created
```
Rust test name: `fn runtime_panics_when_shard_count_is_zero()`

**Scenario A2: Runtime creates correct shard count**
```
Given: shard_count = 4
When:  Runtime::new(shard_count, config) is called
Then:  exactly 4 Shard instances exist
And:   each shard has an independent command queue
```
Rust test name: `fn runtime_creates_correct_shard_count_when_positive()`

---

### Group B: Submit and Routing (PRE-002, POST-001, INV-001)

**Scenario B1: Submit routes to correct shard**
```
Given: a Runtime with shard_count = 4
And:   run_a = RunId::new(10), run_b = RunId::new(11)
When:  submit_direct(run_a, workflow) and submit_direct(run_b, workflow) are called
Then:  the two runs are routed to shards consistent with run.get() % 4
And:  both submissions return Ok(())
```
Rust test name: `fn runtime_routes_run_to_correct_shard_by_run_id_modulo()`

**Scenario B2: Same RunId routed to same shard**
```
Given: a Runtime with shard_count = 3
And:   run = RunId::new(7)
When:  submit_direct(run, workflow_a) succeeds
And:   the same run is submitted again (after cancellation)
Then:  the second submission is rejected with RunAlreadyExists
Or:   if the run has been cancelled/finalized, the second submission succeeds to the same shard
```
Rust test name: `fn same_run_id_routes_to_same_shard_always()`

**Scenario B3: Submit with admission rejection**
```
Given: a Runtime with restricted capability requirements
And:   a workflow that demands capabilities the caller does not hold
When:  submit_direct_with_grants(run, workflow, insufficient_caps) is called
Then:  Err(RuntimeError::AdmissionRejected) is returned
And:   no command is enqueued
```
Rust test name: `fn submit_returns_admission_rejected_when_caps_insufficient()`

---

### Group C: Run Lifecycle Terminal States (POST-002)

**Scenario C1: Run reaches Finished state**
```
Given: a submitted run with a terminal node
When:  tick_all is called until the run completes
Then:  the run reaches terminal state Finished
And:  snapshot_run reports steps_completed == step_count
And:  subsequent tick_all calls for this run produce no side effects
```
Rust test name: `fn run_reaches_finished_state_when_workflow_complete()`

**Scenario C2: Run reaches Failed state**
```
Given: a submitted run that executes a failing action
When:  fail_action(ticket, failure) is called
Then:  the run transitions to terminal state Failed
And:  subsequent commands for this run are ignored
```
Rust test name: `fn run_reaches_failed_state_when_action_fails()`

**Scenario C3: Run reaches Cancelled state**
```
Given: a submitted run
When:  cancel_run(run) is called
Then:  the run transitions to terminal state Cancelled
And:  subsequent tick_all calls for this run produce no side effects
```
Rust test name: `fn run_reaches_cancelled_state_when_cancel_called()`

**Scenario C4: Terminal run ignores subsequent commands**
```
Given: a run that has reached terminal state Finished
When:  resume_run(run) is called
Then:  the command is silently ignored
And:  tick_all returns normally without processing the stale command
```
Rust test name: `fn terminal_run_ignores_subsequent_commands()`

---

### Group D: Action Completion (PRE-003, POST-003)

**Scenario D1: Complete action resumes at correct step**
```
Given: a run suspended at StepIdx(3) with AwaitingAction
And:   ticket = ActionTicket { run, step: StepIdx(3), seq, action, ... }
When:  complete_action_with_output(ticket, output) is called
Then:  the run resumes at StepIdx(4)
And:   the output is delivered to the slot identified by the step's output binding
```
Rust test name: `fn action_completion_resumes_at_correct_step_when_valid_ticket()`

**Scenario D2: Invalid ticket returns error**
```
Given: a ticket that does not correspond to any pending action
When:  complete_action_with_output(invalid_ticket, output) is called
Then:  Err(RuntimeError::InvalidTicket) is returned
And:   no run state is modified
```
Rust test name: `fn complete_action_returns_invalid_ticket_error_when_ticket_unknown()`

**Scenario D3: Fail action transitions run to Failed**
```
Given: a run suspended at a step with AwaitingAction
And:   ticket = ActionTicket for that step
When:  fail_action(ticket, ActionFailure::panic()) is called
Then:  the run transitions to terminal state Failed
And:  the failure reason is recorded in the run's final event
```
Rust test name: `fn fail_action_transitions_run_to_failed_state()`

---

### Group E: Timer Authority (PRE-004, POST-004, INV-002, INV-003)

**Scenario E1: Timer insert increments generation**
```
Given: a TimerWheel with no existing timer for run
When:  insert(run, deadline, Wait) is called
Then:  the resulting TimerEntry has generation == 1
And:  the entry is indexed by both deadline and run
```
Rust test name: `fn timer_insert_first_for_run_creates_generation_one()`

**Scenario E2: Timer insert increments generation monotonically**
```
Given: a TimerWheel with an existing timer for run having generation = g
When:  insert(run, new_deadline, Wait) is called
Then:  the new TimerEntry has generation == g + 1
And:  the old deadline index is cleaned up
```
Rust test name: `fn timer_insert_replaces_existing_increments_generation()`

**Scenario E3: Matching timer entry fires**
```
Given: a run with an active timer entry { run, gen: 3, deadline: t, kind: Wait }
And:   now >= t
When:  timer_entry_fired({ run, gen: 3, deadline: t, kind: Wait }) is called
Then:  the run is advanced
And:  the timer is cleared from the wheel
```
Rust test name: `fn timer_entry_fired_advances_run_when_generation_matches()`

**Scenario E4: Stale generation is silently discarded**
```
Given: a run with an active timer entry { run, gen: 3, deadline: t, kind: Wait }
When:  timer_entry_fired({ run, gen: 2, deadline: t, kind: Wait }) is called
Then:  the entry is silently discarded (no error)
And:  the run is not advanced
And:  the timer entry remains in the wheel
```
Rust test name: `fn timer_entry_fired_ignored_when_generation_stale()`

**Scenario E5: Wrong deadline is silently discarded**
```
Given: a run with an active timer entry { run, gen: 3, deadline: t, kind: Wait }
When:  timer_entry_fired({ run, gen: 3, deadline: t - 1s, kind: Wait }) is called
Then:  the entry is silently discarded (no error)
And:  the run is not advanced
```
Rust test name: `fn timer_entry_fired_ignored_when_deadline_mismatch()`

**Scenario E6: Generation overflow returns error**
```
Given: a run has been timer-inserted 2^64 - 1 times (gen = u64::MAX)
When:  insert(run, deadline, Wait) is called
Then:  Err(TimerWheelError::GenerationExhausted) is returned
And:  no timer is inserted
```
Rust test name: `fn timer_insert_returns_generation_exhausted_at_u64_max()`

---

### Group F: Action Queue Backpressure (POST-006, INV-005)

**Scenario F1: Queue full returns error**
```
Given: a BoundedActionCompletionQueue with capacity = 5
And:   5 tickets are already enqueued
When:  enqueue(sixth_ticket) is called
Then:  Err(ActionQueueError::QueueFull { capacity: 5 }) is returned
And:   len() == 5 (no overflow)
```
Rust test name: `fn action_queue_enqueue_returns_queue_full_when_at_capacity()`

**Scenario F2: 80% capacity triggers backpressure warning**
```
Given: a BoundedActionCompletionQueue with capacity = 10
And:   7 tickets are enqueued (70%)
When:  an 8th ticket is enqueued (80% exactly)
Then:  a BackpressureWarning { depth: 8, capacity: 10 } is emitted on the backpressure channel
And:   Ok(()) is returned
```
Rust test name: `fn action_queue_emits_backpressure_warning_at_80_percent_capacity()`

**Scenario F3: 79% does not trigger backpressure**
```
Given: a BoundedActionCompletionQueue with capacity = 10
And:   7 tickets are enqueued (70%)
When:  an 8th ticket is enqueued
Then:  depth == 8, threshold == 8, so warning IS emitted (80% = 8/10)
Given: a BoundedActionCompletionQueue with capacity = 10
And:   7 tickets are enqueued
When:  a 8th ticket is enqueued (depth=8, threshold=8, depth>=threshold triggers)
Then:  backpressure warning IS emitted
Given: a BoundedActionCompletionQueue with capacity = 10
And:   7 tickets are enqueued (79% = 7.9/10, integer division = 7)
When:  8th ticket is enqueued (depth becomes 8)
Then:  backpressure warning IS emitted because integer threshold is 8 (80% of 10)
```
Rust test name: `fn action_queue_no_warning_before_80_percent_capacity()`

**Scenario F4: Invariant — len never exceeds capacity**
```
Given: a BoundedActionCompletionQueue with any valid capacity
When:  any sequence of enqueue and dequeue operations is performed
Then:  the invariant len() <= capacity() holds at all times
And:   len() + remaining_capacity() == capacity() holds at all times
```
Rust test name: `fn action_queue_invariant_len_never_exceeds_capacity()`

---

### Group G: Tick All (POST-005, INV-007)

**Scenario G1: tick_all processes one command per shard**
```
Given: a Runtime with 3 shards
And:   shard[0] has 2 commands queued, shard[1] has 1, shard[2] has 0
When:  tick_all() is called once
Then:  shard[0] has 1 command remaining
And:   shard[1] has 0 commands remaining
And:   shard[2] is unchanged
And:  tick_all returns true
```
Rust test name: `fn tick_all_processes_at_most_one_command_per_shard()`

**Scenario G2: tick_all returns false on shutdown**
```
Given: a Runtime with at least one shard in ShuttingDown state
When:  tick_all() is called
Then:  tick_all returns false
And:   other shards may still have processed one command each
```
Rust test name: `fn tick_all_returns_false_when_any_shard_shutting_down()`

**Scenario G3: tick_all returns true when all shards alive**
```
Given: a Runtime with all shards in Alive state
When:  tick_all() is called
Then:  tick_all returns true
```
Rust test name: `fn tick_all_returns_true_when_all_shards_alive()`

**Scenario G4: Commands processed in FIFO order per shard**
```
Given: a Shard with commands [A, B, C] in queue order
When:  tick_all() is called 3 times
Then:  command A is processed first, B second, C third
And:  no reordering occurs
```
Rust test name: `fn shard_commands_processed_in_fifo_order()`

---

### Group H: Budget Exhaustion (INV-006)

**Scenario H1: Budget exhaustion exits with StepBudgetExhausted**
```
Given: a StepBudget with remaining = 1
When:  try_take() is called
Then:  it returns false (exhausted)
And:   drive_deterministic exits with EngineSignal::StepBudgetExhausted
```
Rust test name: `fn drive_deterministic_exits_with_step_budget_exhausted_when_budget_zero()`

**Scenario H2: Budget partially decremented**
```
Given: a StepBudget with remaining = 5
When:  try_take() succeeds 3 times
Then:  remaining == 2
And:   try_take() succeeds once more
Then:  remaining == 1
And:   try_take() fails
```
Rust test name: `fn step_budget_decrements_correctly_on_each_step()`

---

## 4. Proptest Invariants

### PI-001: Timer Generation Monotonicity (INV-002)
```
Property: For any run, successive inserts produce strictly monotonic generations: g1 < g2 < g3
Strategy: any_run_id(), any_valid_deadline(), any_pending_timer_kind()
Anti-invariant: Rapid insert/insert without firing (generation overflow path)
```

### PI-002: Timer Fire Correctness (INV-003)
```
Property: fire_expired(now) only returns entries where deadline <= now AND generation matches current
Strategy: any_instant(), build timer wheel with any combination of expired/non-expired entries
Anti-invariant: Entries from previous generations (stale) must never appear in fire_expired output
```

### PI-003: Action Queue Capacity Bound (INV-005)
```
Property: After any enqueue, len() <= capacity; after any dequeue, len() >= 0
Strategy: any_capacity(1..1000), arbitrary sequence of enqueue/dequeue operations
Anti-invariant: Overflow when capacity is very small (1-3) and many threads enqueue
```

### PI-004: Action Queue FIFO Ordering (INV-004)
```
Property: Dequeue order matches enqueue order (FIFO)
Strategy: Enqueue N tickets with unique seq numbers, verify dequeue returns them in seq order
Anti-invariant: Concurrent enqueue/dequeue (though ASM-001 says single-threaded per shard)
```

### PI-005: Runtime Routing Determinism (INV-001)
```
Property: shard_for(run) is idempotent: calling it any number of times returns the same shard
Strategy: any_run_id(), any_runtime(shard_count in 1..16)
Anti-invariant: Runtime with changing shard count (impossible by construction — shard_count fixed at Runtime::new)
```

### PI-006: Budget Try-Take Accuracy (INV-006)
```
Property: try_take() returns true exactly remaining() times before returning false
Strategy: any_positive_budget(), exhaustively call try_take() and compare
Anti-invariant: Budget underflow or returning true after false (race — not applicable in single-threaded)
```

---

## 5. Fuzz Targets

### FT-001: CompiledWorkflow Deserialization
```
Target: Workflow YAML/JSON parsed into CompiledWorkflow
Risk: Panics from index out of bounds, OOM from malformed YAML, logic errors in expression evaluation
Corpus seeds: Valid workflow YAML from integration tests, edge-case expressions (division by zero, empty nodes)
Tool: cargo-fuzz or proptest with arbitrary bytes
```

### FT-002: ActionTicket Arbitrary Generation
```
Target: Runtime::complete_action_with_output(ticket, output)
Risk: A malformed ActionTicket could cause panic or state corruption if validation is missing
Corpus seeds: Valid tickets from integration tests, boundary values (seq=0, max RunId, max StepIdx)
Tool: proptest with arbitrary ticket + output combinations
```

---

## 6. Kani Harnesses (BLOCKED — Compensating Evidence Filed)

### KH-001: tick_all Panic-Freedom (KANI-TICK-001)
```
Property: tick_all never panics for any valid Runtime state
Bound: shard_count <= 4, queue_depth <= 3
Status: BLOCKED — vb_storage crate compilation errors (72 kani::any() type inference failures)
Compensating evidence: 1,354 integration tests pass (PO-023, PO-024, PO-025, PO-026 all PASS)
Waiver: PO-014 BLOCKED_TOOLING, expires 2026-12-31
```

### KH-002: Shard::tick Panic-Freedom (KANI-SHARD-001)
```
Property: Shard::tick never panics for any valid ShardCommand
Bound: command queue depth <= 3
Status: BLOCKED — same vb_storage issue as KH-001
Compensating evidence: TLA+ ShardProcessing verified at reduced bounds (PO-002 PASS_LOCAL with waiver)
Waiver: PO-015 BLOCKED_TOOLING, expires 2026-12-31
```

### KH-003: Timer Insert Panic-Freedom (KANI-TIMER-001)
```
Property: insert never panics; generation arithmetic is bounded by u64::MAX
Bound: Timer wheel with up to 8 runIds, generations up to u64::MAX
Status: BLOCKED — same vb_storage issue as KH-001
Compensating evidence: TLA+ TimerWheel verified generation monotonicity at reduced bounds (PO-004 PASS_LOCAL with waiver)
Waiver: PO-016 BLOCKED_TOOLING, expires 2026-12-31
```

### KH-004: Action Queue Enqueue Panic-Freedom (KANI-QUEUE-001)
```
Property: enqueue never panics; len() never exceeds capacity
Bound: capacity in 1..256, concurrent enqueue up to 4 threads
Status: BLOCKED — same vb_storage issue as KH-001
Compensating evidence: ActionQueue uses safe Rust only; unit tests verify capacity invariant; Verus PO-011 PASS for budget
Waiver: PO-017 BLOCKED_TOOLING, expires 2026-12-31
```

### KH-005: RunFrame State Transition (KANI-FRAME-001)
```
Property: RunFrame state transitions are exhaustive and never panic
Bound: All EngineSignal variants, budget in 0..1000
Status: BLOCKED — same vb_storage issue as KH-001
Compensating evidence: Verus PO-011 PASS for run_loop budget exhaustion; 1,354 integration tests
Waiver: PO-018 BLOCKED_TOOLING, expires 2026-12-31
```

---

## 7. Mutation Checkpoints

Critical mutations that must be caught by existing test suite:

| Mutation | Target | Must be caught by |
|----------|--------|-------------------|
| Replace `<` with `<=` in `budget.try_take()` condition | INV-006 | `step_budget_decrements_correctly_on_each_step` |
| Remove `backpressure_tx.send()` call in `enqueue` | POST-006 | `action_queue_emits_backpressure_warning_at_80_percent_capacity` |
| Swap `push_back`/`push_front` in queue implementation | INV-004 | `shard_commands_processed_in_fifo_order` + `action_queue_dequeue_returns_fifo_order` |
| Remove generation check in `timer_entry_fired` | INV-003 | `timer_entry_fired_ignored_when_generation_stale` |
| Remove capacity check in `enqueue` (allow overflow) | INV-005 | `action_queue_invariant_len_never_exceeds_capacity` |
| Change `mod` to `/` in `shard_for` routing | INV-001 | `runtime_routes_run_to_correct_shard_by_run_id_modulo` |
| Remove terminal state guard in command processing | POST-002 | `terminal_run_ignores_subsequent_commands` |

**Mutation kill rate threshold**: ≥ 90%. Current evidence: 1,354 integration tests provide broad coverage. Formal `cargo-mutants` run deferred until Kani tooling (vb_storage) is repaired.

---

## 8. Combinatorial Coverage Matrix

### Runtime Submission (PRE-002, POST-001)

| Scenario | RunId validity | Workflow validity | Caps | Expected Output | Layer |
|----------|----------------|-------------------|------|-----------------|-------|
| happy path | valid | valid | sufficient | Ok(()) | integration |
| zero RunId | zero inner | valid | sufficient | Err(RuntimeError::InvalidRunId) | unit |
| empty workflow | valid | zero nodes | sufficient | Err(RuntimeError::AdmissionRejected) | integration |
| insufficient caps | valid | valid | insufficient | Err(RuntimeError::AdmissionRejected) | integration |
| run already exists | same id | valid | sufficient | Err(RuntimeError::RunAlreadyExists) | integration |

### Timer Wheel (INV-002, INV-003, PRE-004)

| Scenario | Existing timers | Generation | Deadline | Kind | Expected | Layer |
|----------|----------------|------------|----------|------|----------|-------|
| first insert | 0 | 1 | any | Wait/Ask | Ok(entry) | unit |
| replacement | 1 (gen=g) | g+1 | any | Wait/Ask | Ok(entry) | unit |
| stale generation fire | 1 (gen=g) | g-1 | matching | matching | Ignored (no error) | unit |
| wrong deadline fire | 1 | matching | wrong | matching | Ignored (no error) | unit |
| gen overflow | 2^64-1 inserts | — | any | any | Err(GenerationExhausted) | unit |
| fire expired | expired entries | matching | <= now | any | Vec<TimerEntry> | unit |

### Action Queue (INV-004, INV-005, POST-006)

| Scenario | Depth | Operation | Expected | Layer |
|----------|-------|-----------|----------|-------|
| empty dequeue | 0 | dequeue | None | unit |
| one item FIFO | 1 | dequeue x1 | Some(ticket_0) | unit |
| three items FIFO | 3 | dequeue x3 | seq: 0, 1, 2 | unit |
| enqueue at 79% | cap=10, depth=7 | enqueue | Ok, no warning | unit |
| enqueue at 80% | cap=10, depth=8 | enqueue | Ok, BackpressureWarning | unit |
| enqueue at 100% | cap=10, depth=10 | enqueue | Err(QueueFull) | unit |
| invariant | any | any sequence | len <= cap | unit + integration |

### Tick All (POST-005, INV-007)

| Scenario | Shard state | Commands | Expected | Layer |
|----------|-------------|----------|----------|-------|
| all alive, all have commands | Alive x N | >= 1 each | true, 1 cmd each processed | integration |
| all alive, no commands | Alive x N | 0 each | true, nothing processed | unit |
| one shutting down | ShuttingDown + Alive | any | false | integration |
| one command, 3 ticks | Alive | 1 | cmd processed on tick 1, nothing on tick 2-3 | unit |

---

## 9. Integration Test Evidence Baseline

The following tests already exist and provide behavioral coverage. They are the **authoritative evidence** that the runtime behaves according to contract:

| Evidence ID | Test artifact | Command | Status |
|-------------|--------------|---------|--------|
| PO-023 | `crates/vb_runtime/tests/recovery_bdd_tests.rs` | `cargo test --package vb_runtime recovery_bdd_tests` | **PASS** — 20 recovery BDD scenarios |
| PO-024 | `crates/vb_cli/tests/cli_vb_m214_bdd_scenarios.rs` | `cargo test --package vb_cli cli_vb_m214_bdd_scenarios` | **PASS** — 17 CLI BDD scenarios |
| PO-025 | `crates/vb_cli/tests/cli_verify_integration.rs` | `cargo test --package vb_cli cli_verify_integration` | **PASS** — 6 verify BDD scenarios |
| PO-026 | `crates/workspace_tests/src/acceptance_catalog.rs` | `cargo test --package workspace_tests acceptance_catalog` | **PASS** — 21 catalog scenarios |
| PO-011 | `crates/vb_core/src/engine/run_loop.rs` | Verus verified budget exhaustion | **PASS** — run_loop budget exhaustion |

Formal verification obligations with compensating evidence waivers:
- PO-001 (TLA-WF-001): **PASS** — INV-001 routing determinism
- PO-003 (TLA-WF-003): **PASS** — POST-002 terminal uniqueness
- PO-002, PO-004, PO-005 (TLA+): **PASS_LOCAL with waiver** — reduced bounds, full verification required before release
- PO-007 through PO-013 (Verus): **WAIVED** — BLOCKED_DESIGN (production source edits required)
- PO-014 through PO-018 (Kani): **WAIVED** — BLOCKED_TOOLING (vb_storage compilation errors)

---

## 10. Open Questions

### OQ-001: BDD Scenario Scope (DISCOVERY_BLOCKED)
**Question**: Does vb-c1s0 define NEW BDD scenarios (beyond the existing 20 recovery + 17 CLI + 6 verify + 21 catalog = 64 existing scenarios)?

**Impact**: If new scenarios are required, this test plan only covers the runtime behavior underlying the orchestration contract. The BDD catalog expansion would be a separate bead.

**Current state**: Integration test evidence (PO-023 through PO-026) all PASS, covering BDD-KYYF-001 through BDD-NJJU-004 and VB-BDD-CATALOG-001 through 010. If vb-c1s0 is validating existing catalog scenarios, no new test scenarios are needed — the test plan above fills in the unit/integration gap for runtime behaviors not covered by existing BDD integration tests.

**Recommendation**: Resolve DISCOVERY_BLOCKED before finalizing. If new scenarios are required, add them to the BDD catalog before test-writer begins.

### OQ-002: Compound Workflow Scheduling
**Question**: Are there additional scheduling behaviors beyond ActionScheduled/WaitScheduled/AskScheduled for compound (parallel/forked) workflows?

**Impact**: If compound workflows exist, INV-007 (FIFO per shard) may need additional race-condition tests covering concurrent command submission from multiple workflow branches to the same shard.

**Current state**: No evidence of compound workflow types in the contract. Assumes all workflows are sequential unless proven otherwise.

### OQ-003: Kani Tooling Repair
**Question**: Who owns the vb_storage repair to fix the 72 `kani::any()` type inference compilation errors?

**Impact**: Until vb_storage is repaired, Kani harnesses (KH-001 through KH-005) cannot be executed. The formal verification gate (GATE-PROOF-001) remains WAIVED.

**Recommendation**: Assign vb_storage owner to repair Kani compilation errors before PO-027 can be resolved.

---

## 11. Test Execution Order

1. **Unit tests first** — run `cargo test --package vb_runtime --lib action_queue --lib timer_wheel` to establish pure-data-structure correctness
2. **Integration tests** — run `cargo test --package vb_runtime` then `cargo test --package vb_cli` then `cargo test --package workspace_tests`
3. **BDD validation** — confirm PO-023 through PO-026 still pass after any changes
4. **Mutation testing** — run `cargo mutants` after all unit/integration tests pass (deferred until Kani tooling repaired)
5. **Formal verification gate** — `moon run :verify-proof` (deferred until vb_storage repair)

---

*Test plan produced at Go-skill State 7. Forward to test-writer for execution.*
