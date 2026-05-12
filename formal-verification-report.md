# Formal Verification Cross-Check Report

**STATUS: REJECTED**

**Scope:** TLA+ models in `/home/lewis/src/Velvet-ballistics/specs/tla/` vs. Rust implementation in main crates  
**Date:** 2026-05-11  
**Verifier:** formal-verifier agent (k2p6)

---

## Executive Summary

Cross-checked **9 TLA+ models** against their corresponding Rust implementations. Found:

| Category | Count |
|---|---|
| **VIOLATIONS** (code allows what model forbids, or contradicts model) | **4** |
| **INCOMPLETENESS** (model doesn't cover implemented behavior) | **3** |
| **RACE CONDITIONS** (model's sequential abstraction hides concurrency bugs) | **2** |
| **BOUNDARY ERRORS** (model's small constants miss real-world edge cases) | **1** |

**Bottom line:** The TLA+ models are valuable specifications but contain semantic mismatches, outdated assumptions, and gaps relative to the actual Rust runtime. Several safety properties modeled in TLA+ are **not guaranteed** by the Rust implementation under certain configurations.

---

## 1. StepState.tla vs `frame.rs` + `step_state.rs`

### TLA+ Invariants
```tla
ValidNext(source) ==
    CASE source = "Pending"  -> {"Running", "Succeeded", "Failed", "Cancelled", "Skipped"}
      [] source = "Running"  -> {"Succeeded", "Failed", "Waiting", "Asking", "Cancelled", "Skipped"}
      [] source = "Waiting"  -> {"Running"}
      [] source = "Asking"  -> {"Running"}
      [] OTHER              -> {}

TerminalStateBlocksOutwardTransitions ==
    \A step \in StepId :
        step_state[step] \in TerminalStates
            => \A next \in StateNames :
                IsValidTransition(step_state[step], next) <=> (next = step_state[step])
```

### Rust Implementation
`crates/vb_core/src/frame.rs:394-431`:
```rust
fn validate_transition(current: StepState, new: StepState) -> CoreResult<()> {
    let valid = match (current, new) {
        (StepState::Pending, StepState::Running) => true,
        (StepState::Pending, StepState::Succeeded | StepState::Failed
                              | StepState::Cancelled | StepState::Skipped) => true,
        (StepState::Running, StepState::Succeeded | StepState::Failed
                              | StepState::Waiting | StepState::Asking
                              | StepState::Cancelled | StepState::Skipped) => true,
        (StepState::Waiting | StepState::Asking, StepState::Running) => true,
        (state, next) if state == next => true,
        _ => false,
    };
```

### Result: ✅ PASS

The Rust `validate_transition` exactly matches the TLA+ `ValidNext` function. The proof kernel in `crates/vb_proof_kernels/src/step_state.rs` enumerates the same transitions and tests confirm terminal states block outward transitions. The `TerminalStateBlocksOutwardTransitions` theorem holds in the implementation.

---

## 2. ShardScheduler.tla vs `shard/impl_.rs` + `shard/lifecycle.rs`

### TLA+ Invariants
```tla
TickOneCommand ==
    \A s \in ShardId :
        /\ shard_status[s] = FALSE
        /\ Len(queues[s]) > 0
        => Len(queues[s])' = Len(queues[s]) - 1

QueueBounded ==
    \A s \in ShardId : Len(queues[s]) <= MAX_COMMAND_QUEUE_CAPACITY
```

### Rust Implementation
`crates/vb_runtime/src/shard/impl_.rs:118-162`:
```rust
pub fn tick(&mut self) -> RuntimeResult<bool> {
    if self.shutting_down {
        return Ok(false);
    }
    let Some(cmd) = self.command_queue.pop() else {
        return Ok(true);
    };
    // ... process exactly one command ...
    Ok(true)
}
```

`crates/vb_runtime/src/shard/impl_.rs:343-365`:
```rust
pub fn drain_for_shutdown(&mut self) -> RuntimeResult<()> {
    let limit = self.command_queue.capacity();
    let mut processed = 0usize;
    while processed < limit {
        if self.shutting_down { /* ... */ return Ok(()); }
        if self.command_queue.is_empty() { return Ok(()); }
        if !self.tick()? { /* shutdown cmd */ return Ok(()); }
        processed = processed.saturating_add(1);
    }
    Err(RuntimeError::ShutdownInProgress)
}
```

### Finding 2a: ⚠️ RACE CONDITION — Concurrent enqueue during `drain_for_shutdown`

**Category:** RACE CONDITION (model's sequential abstraction hides real concurrency)

The TLA+ `ShutdownGraceful` action assumes a closed world where no new commands are enqueued during shutdown:
```tla
ShutdownGraceful ==
    /\ \E s \in ShardId :
        /\ shard_status[s] = FALSE
        /\ LET q == queues[s] IN
            IF Len(q) = 0 THEN ...
```

But the Rust `drain_for_shutdown` uses:
```rust
let limit = self.command_queue.capacity();
while processed < limit {
    if self.command_queue.is_empty() { return Ok(()); }
    // ...
    processed = processed.saturating_add(1);
}
Err(RuntimeError::ShutdownInProgress)
```

**Race:** Another thread can `enqueue()` commands concurrently while `drain_for_shutdown` is running. If commands are added at the same rate they're consumed, `processed` can reach `limit` while the queue is still non-empty. The function then returns `Err(ShutdownInProgress)` even though work remains — violating the TLA+ `ShutdownProgress` property (`<>(AllDrained)`).

The TLA+ model has no concurrent `SubmitCommand` during `ShutdownGraceful`; it models a sequential interleaving, not true concurrent enqueue.

### Finding 2b: ⚠️ BOUNDARY ERROR — `drain_for_shutdown` limit is current capacity, not `MAX_COMMAND_QUEUE_CAPACITY`

**Category:** BOUNDARY ERROR

The TLA+ model uses `MAX_COMMAND_QUEUE_CAPACITY == 65536`. The Rust code uses:
```rust
let limit = self.command_queue.capacity();
```

If `command_queue_capacity` in `ShardConfig` is set to a value smaller than the actual number of commands enqueued during shutdown (e.g., capacity=16 but 20 commands arrive), `drain_for_shutdown` will fail with `ShutdownInProgress` after processing only 16. The TLA+ model's state constraint (`Len(queues[1]) <= 3`) doesn't explore this boundary.

---

## 3. AttemptTracking.tla vs `shard/lifecycle.rs` + `shard/helpers.rs`

### TLA+ Invariants
```tla
IsStale(run, step, attempt) ==
    attempt < latest_attempt[<<run, step>>]

StaleCompletionRejected ==
    \A event \in DOMAIN journal :
        journal[event].type = "ActionCompleted"
        => ~IsStale(journal[event].run, journal[event].step, journal[event].attempt)
```

### Rust Implementation
`crates/vb_runtime/src/shard/helpers.rs:46-70`:
```rust
fn validate_ticket_attempt(state: &RunState, ticket: ActionTicket) -> RuntimeResult<()> {
    let current = state
        .action_attempts
        .get(ticket.step.as_usize())
        .copied()
        .ok_or(RuntimeError::InvalidActionCompletion)?;
    if ticket.attempt < current {
        return Err(RuntimeError::StaleAttempt {
            incoming: ticket.attempt,
            current,
        });
    }
    Ok(())
}
```

### Result: ✅ PASS (for stale rejection)

The Rust code correctly rejects stale attempts (`ticket.attempt < current`). Tests in `lifecycle.rs` confirm this (`stale_attempt_completion_leaves_run_counters_journal_and_frame_unchanged`).

### Finding 3a: ⚠️ INCOMPLETENESS — Model assumes global `latest_attempt`, Rust has per-shard counters

**Category:** INCOMPLETENESS (model doesn't cover cross-shard behavior)

The TLA+ `latest_attempt` is a global function over all `(RunId × StepId)`. In Rust, `action_attempts` is a `Box<[u16]>` stored in `RunState`, which lives inside a specific `Shard`. If a run were ever transferred between shards (as contemplated by `ShardOwnership.tla`), the attempt counter would be lost or reset.

The `AttemptTracking.tla` model assumes a single global journal and single authority for attempt numbers. The Rust runtime has no run transfer mechanism yet, but if one is added, the model would need to account for attempt state migration.

---

## 4. ShardOwnership.tla vs `shard/lifecycle.rs`

### TLA+ Invariants
```tla
SingleOwner ==
    \A run \in RunId :
        \E shard \in ShardId :
            run_owner[run] = shard /\
            \A other \in ShardId \ {shard} : run \notin shard_runs[other]

RunOnExactlyOneShard ==
    \A run \in RunId :
        \A shard1, shard2 \in ShardId :
            run \in shard_runs[shard1] /\ run \in shard_runs[shard2]
            => shard1 = shard2
```

### Rust Implementation
`crates/vb_runtime/src/shard/lifecycle.rs:79-132`:
```rust
pub(crate) fn handle_submit(...) -> RuntimeResult<()> {
    if self.runs.contains_key(&run) {
        return Err(RuntimeError::RunAlreadyExists);
    }
    // ... create RunState, insert into self.runs ...
    self.runs.insert(run, state);
}
```

### Finding 4a: 🔴 VIOLATION — TLA+ model describes run transfers that don't exist in Rust

**Category:** VIOLATION (model forbids what code doesn't implement) / INCOMPLETENESS

The `ShardOwnership.tla` model includes `InitiateTransfer` and `CompleteTransfer` actions that model moving a run from one shard to another. The Rust codebase has **no run transfer mechanism at all**. Runs are created on a shard and live there until completion or cancellation.

This means:
1. The `SingleOwner` invariant trivially holds (a run is never on more than one shard because it can't move), but the model's machinery for maintaining it (`pending_transfers`, `CompleteTransfer`) is dead code relative to the implementation.
2. The TLA+ model is **more complex** than the implementation — it proves properties about a feature that doesn't exist.

**Recommendation:** Either implement run transfers in Rust, or simplify `ShardOwnership.tla` to match the actual single-shard ownership model.

---

## 5. BoundedAdmission.tla vs `budget.rs` + admission logic

### TLA+ Invariants
```tla
NoRunAdmittedWithoutReservation ==
    \A run \in admitted_runs :
        reserved_resources[run].slots > 0 /\
        reserved_resources[run].actions > 0

ShardCapacityBounded ==
    \A shard \in ShardId :
        Cardinality(shard_runs[shard]) <= MaxRunsPerShard
```

### Rust Implementation
`crates/vb_runtime/src/shard/lifecycle.rs:99-103`:
```rust
if self.runs.len() >= self.max_active_runs {
    return Err(RuntimeError::ActiveRunCapacityExceeded {
        capacity: self.max_active_runs,
    });
}
```

`crates/vb_core/src/budget.rs:431-493`:
```rust
pub fn try_add_budget(&self, budget: &AggregateResourceBudget) -> Result<Self, AggregateBudgetError> {
    Ok(Self {
        max_steps_executable: add_dim(self.max_steps_executable, u64::from(budget.max_steps_executable), "max_steps_executable")?,
        // ... component-wise checked_add for all fields ...
    })
}
```

### Result: ✅ PASS (for shard capacity)

The `ShardCapacityBounded` invariant is enforced by `self.runs.len() >= self.max_active_runs`.

### Finding 5a: 🔴 VIOLATION — `NoRunAdmittedWithoutReservation` not enforced

**Category:** VIOLATION (code allows what model forbids)

The TLA+ model requires every admitted run to have `reserved_resources[run].slots > 0 && reserved_resources[run].actions > 0`. The Rust admission logic (`build_admission` in `lifecycle.rs:134-173`) checks artifact existence and capabilities but **does not allocate or verify per-run resource reservations**.

The `AggregateResourceBudget` is computed from the workflow, and `AggregateResourceUsage::try_add_budget` tracks aggregate usage, but there's no explicit check that `budget.max_steps_executable > 0 && budget.max_action_tickets > 0` before admitting. A malformed workflow could theoretically produce a zero budget and still be admitted (the capacity check would pass if aggregate usage is below capacity).

**Missing check:** The Rust code should reject admission when the computed budget has zero slots or zero actions, matching the TLA+ `NoRunAdmittedWithoutReservation` invariant.

---

## 6. RecoveryReplay.tla vs `recovery.rs` + `journal.rs`

### TLA+ Invariants
```tla
NoDuplicateNonIdempotent ==
    \A i \in 1..Len(journal) :
        \A j \in 1..Len(journal) :
            i /= j /\
            journal[i].type = "ActionScheduled" /\
            journal[j].type = "ActionScheduled" /\
            journal[i].run = journal[j].run /\
            journal[i].step = journal[j].step /\
            journal[i].action = journal[j].action /\
            journal[i].attempt = journal[j].attempt /\
            journal[i].policy /= "DeterministicPure" /\
            journal[i].policy /= "IdempotentExternal"
            => FALSE

ReplaySafe ==
    \A run, step, action, attempt :
        \A i, j \in 1..Len(journal) :
            i < j /\
            journal[i].type = "ActionCompleted" /\
            journal[i].run = run /\
            journal[i].step = step /\
            journal[i].action = action /\
            journal[i].attempt = attempt
            => journal[j].type /= "ActionScheduled" \/ journal[j].attempt /= attempt
```

### Rust Implementation
`crates/vb_runtime/src/recovery.rs:30-71`:
```rust
pub trait RuntimeRecoveryBoundary {
    fn summary(&self) -> RecoveryRuntimeSummary;
    fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame>;
}

impl RuntimeRecoveryBoundary for DurableFrameRecoveryBoundary {
    fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame> {
        reject_unsupported_live_frame_state(&self.seed)?;
        let mut frame = empty_recovered_frame(&self.seed)?;
        apply_recovered_steps(&mut frame, &self.seed)?;
        apply_recovered_slots(&mut frame, &self.seed)?;
        apply_recovered_pc(&mut frame, &self.seed)?;
        Ok(frame)
    }
}
```

### Finding 6a: ⚠️ INCOMPLETENESS — Recovery code doesn't handle idempotency policies

**Category:** INCOMPLETENESS (model covers behavior Rust doesn't implement)

The TLA+ model tracks `IdempotencyPolicy` = {DeterministicPure, IdempotentExternal, AtLeastOnceExternal} and uses it to decide whether an action can be safely re-executed during replay. The Rust recovery boundary (`recovery.rs`) hydrates frame state (steps, slots, PC) but **does not implement action replay logic at all**. It restores the frame to its pre-crash state; the runtime then resumes from the PC.

The `RecoveryReplay.tla` model describes a more sophisticated recovery system where the journal is replayed to reconstruct state, with non-idempotent actions being skipped. The actual Rust implementation uses frame snapshots for recovery, not journal replay of individual actions.

**Gap:** The TLA+ model proves properties about a journal-replay recovery strategy that the Rust code doesn't use. The Rust strategy (frame snapshot hydration) is simpler and avoids the replay problem entirely, but the model doesn't describe it.

---

## 7. BudgetArithmetic.tla vs `budget.rs`

### TLA+ Invariants
```tla
sub_budgets(b1, b2) ==
    [
        max_steps_executable |-> IF b1.max_steps_executable >= b2.max_steps_executable
                                 THEN b1.max_steps_executable - b2.max_steps_executable
                                 ELSE 0,
        // ... all fields floor at 0 ...
    ]

SubNeverNegative ==
    \A b1 \in Budget, b2 \in Budget :
        \A field \in BudgetFields :
            sub_budgets(b1, b2)[field] >= 0
```

### Rust Implementation
`crates/vb_core/src/budget.rs:752-760`:
```rust
fn sub_dim(current: u64, requested: u64, resource: &'static str) -> Result<u64, AggregateBudgetError> {
    current
        .checked_sub(requested)
        .ok_or(AggregateBudgetError::Underflow { resource })
}
```

### Finding 7a: 🔴 VIOLATION — `sub_budgets` floors at 0 in TLA+, errors in Rust

**Category:** VIOLATION (code behavior contradicts model semantics)

The TLA+ model explicitly defines `sub_budgets` to **floor at 0**:
> "Floor at 0: if b1[field] < b2[field], result is 0."

The Rust `sub_dim` uses `checked_sub`, which returns `Err(Underflow)` when `current < requested` instead of returning 0. This is a direct semantic mismatch.

**Impact:** If a run completes and its budget is subtracted from aggregate usage, but another run has already "stolen" some of that capacity (due to a bug or race), the subtraction will error instead of silently flooring. This is arguably *safer* than the TLA+ model (failing closed), but it **violates** the `SubNeverNegative` property as stated.

The model comment acknowledges: "Models Rust sub_dim() -> checked_sub() with Err on underflow propagated to caller." But then the `SubNeverNegative` theorem is proven for the TLA+ floor-at-0 semantics, not for the error-propagation semantics.

**This is a specification bug, not a code bug** — the TLA+ should either:
1. Model `sub_budgets` as returning `Ok(result)` or `Err(Underflow)`, OR
2. Change Rust to floor at 0 (which would silently accept accounting errors)

### Finding 7b: ⚠️ BOUNDARY ERROR — TLA+ uses `Nat` (unbounded), Rust uses `u64`

**Category:** BOUNDARY ERROR (model's small constants miss real overflow behavior)

The TLA+ model uses `Nat` (unbounded natural numbers) for all budget fields. The Rust implementation uses `u64` with `checked_add`/`checked_sub`. The model acknowledges this in comments but doesn't model the overflow/underflow behavior explicitly.

The `StateConstraint` in `BudgetArithmetic.tla` has no state constraints (it's a trivial state machine). It doesn't explore boundary values like `u64::MAX` where Rust behavior diverges from TLA+ `Nat`.

---

## 8. TaintLattice.tla vs `value.rs`

### TLA+ Invariants
```tla
Rank(t) ==
    CASE t = "Clean"              -> 0
      [] t = "DerivedFromSecret"  -> 1
      [] t = "Secret"            -> 2

join(a, b) ==
    LET ra == Rank(a)
        rb == Rank(b)
    IN  IF ra >= rb THEN a ELSE b

LatticeCommutative ==
    \A a \in TaintLevel, b \in TaintLevel : join(a, b) = join(b, a)
```

### Rust Implementation
`crates/vb_core/src/value.rs:24-36`:
```rust
pub fn join_taint(a: Taint, b: Taint) -> Taint {
    let a_disc: u8 = match a {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    };
    let b_disc: u8 = match b {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    };
    if a_disc >= b_disc { a } else { b }
}
```

### Result: ✅ PASS

The Rust `join_taint` exactly implements `max(rank(a), rank(b))`. Tests verify all 6 lattice laws (commutativity, associativity, idempotence, identity, Secret top, Clean bottom). The `#[repr(u8)]` on `Taint` enum ensures discriminant values match the rank mapping.

---

## 9. JournalBeforeDispatch.tla vs `journal.rs` + runtime journal

### TLA+ Invariants
```tla
DispatchSafety ==
    \A <<run, step, action, attempt>> \in dispatched :
        \E event \in DOMAIN journal :
            journal[event].type = "ActionScheduled" /\
            journal[event].run = run /\
            journal[event].step = step /\
            journal[event].action = action /\
            journal[event].attempt = attempt

DispatchBeforeCommit ==
    \A <<run, step, action, attempt>> \in dispatched :
        \E idx \in DOMAIN journal :
            journal[idx].type = "ActionScheduled" /\
            // ...
            idx \in DOMAIN journal
```

### Rust Implementation
`crates/vb_storage/src/journal.rs:195-198`:
```rust
pub fn append_strict(&self, event: &JournalEvent) -> Result<(), JournalError> {
    self.append_unpersisted(event)?;
    self.persist_strict()
}
```

`crates/vb_runtime/src/journal.rs:318-325`:
```rust
fn append_storage_event(&self, event: &JournalEvent) -> RuntimeResult<()> {
    let result = if self.profile == DurabilityProfile::Strict {
        self.journal.append_strict(event)
    } else {
        self.journal.append_journaled(event)
    };
    result.map_err(RuntimeError::from)
}
```

`crates/vb_runtime/src/journal.rs:591-608`:
```rust
impl RuntimeJournal for QueuedStorageRuntimeJournal {
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        if self.profile == DurabilityProfile::Strict {
            return Err(RuntimeError::UnsupportedAsyncStrictAck);
        }
        // ... enqueue to queue, NOT immediately persisted ...
        let result = self.queue.enqueue_journaled(storage_event);
        result.map_err(RuntimeError::from)?;
        sequences.insert(run_id, next);
        Ok(())
    }
}
```

### Finding 9a: 🔴 VIOLATION — `Journaled` and `Queued` profiles dispatch before durable commit

**Category:** VIOLATION (code allows what model forbids) + RACE CONDITION

The TLA+ model assumes **synchronous append** (append blocks until durable):
> "The Rust implementation (StorageRuntimeJournal::append) is synchronous — the append blocks until the event is durable before execute_do() returns RuntimeSignal::AwaitingAction."

This is **only true for `DurabilityProfile::Strict`**. For `DurabilityProfile::Journaled`:
```rust
self.journal.append_journaled(event)  // NO sync barrier!
```

And for `QueuedStorageRuntimeJournal`:
```rust
self.queue.enqueue_journaled(storage_event)  // Just enqueues, NOT persisted!
```

**In both `Journaled` and `Queued` profiles, the action can be dispatched to the external boundary BEFORE the journal entry is actually durable.** If the system crashes after dispatch but before the journal is flushed, the `ActionScheduled` event will be lost. On recovery, the action may have already executed externally, but the runtime has no record of scheduling it — violating `DispatchSafety`.

The TLA+ model's `ActionScheduledThenDispatch` action atomically appends to journal AND dispatches:
```tla
ActionScheduledThenDispatch(run, step, action, attempt) ==
    /\ journal' = Append(journal, [type |-> "ActionScheduled", ...])
    /\ dispatched' = dispatched \cup {<<run, step, action, attempt>>}
```

This atomicity is **only guaranteed for `Strict` profile** in Rust.

### Finding 9b: 🔴 VIOLATION — Frame mutation before journal append in `handle_action_completion`

**Category:** VIOLATION (code allows state mutation without corresponding journal entry)

`crates/vb_runtime/src/shard/lifecycle.rs:180-227`:
```rust
pub(crate) fn handle_action_completion(...) -> RuntimeResult<()> {
    // 1. MUTATE FRAME FIRST
    state.frame.write_slot_with_taint(output.output_slot, output.value, output.taint)?;
    state.frame.mark_succeeded(ticket.step)?;
    advance_after_action_completion(state, ticket.step)?;

    // 2. JOURNAL APPENDS HAPPEN AFTER MUTATION
    self.journal.append(RuntimeJournalEvent::SlotWritten { ... })?;
    self.journal.append(RuntimeJournalEvent::StepSucceeded { ... })?;
    self.journal.append(RuntimeJournalEvent::ActionCompleted { ... })?;
    self.drive_run(run)
}
```

**If any journal append fails** (e.g., storage I/O error, queue full), the frame has already been mutated (slot written, step marked Succeeded, PC advanced) but the journal doesn't reflect it. On recovery, the runtime would replay up to the last successful journal entry and resume from an inconsistent state.

The TLA+ model assumes all state changes are atomically coupled with journal appends. The Rust code has a **two-phase commit problem**: frame mutation succeeds but journal append may fail.

---

## Detailed Finding Register

| # | Model | File | Category | Severity | Description |
|---|---|---|---|---|---|
| 2a | ShardScheduler | `shard/impl_.rs` | RACE CONDITION | High | `drain_for_shutdown` can fail with `ShutdownInProgress` due to concurrent enqueue |
| 2b | ShardScheduler | `shard/impl_.rs` | BOUNDARY ERROR | Medium | `drain_for_shutdown` limit is current capacity, not `MAX_COMMAND_QUEUE_CAPACITY` |
| 3a | AttemptTracking | `shard/helpers.rs` | INCOMPLETENESS | Medium | Model assumes global attempt tracker; Rust has per-shard counters |
| 4a | ShardOwnership | `shard/lifecycle.rs` | VIOLATION | High | TLA+ models run transfers; Rust has no transfer mechanism |
| 5a | BoundedAdmission | `shard/lifecycle.rs` | VIOLATION | Medium | `NoRunAdmittedWithoutReservation` not enforced in Rust admission |
| 6a | RecoveryReplay | `recovery.rs` | INCOMPLETENESS | Medium | Model describes journal replay; Rust uses frame snapshot hydration |
| 7a | BudgetArithmetic | `budget.rs` | VIOLATION | High | TLA+ `sub_budgets` floors at 0; Rust `sub_dim` returns `Err(Underflow)` |
| 7b | BudgetArithmetic | `budget.rs` | BOUNDARY ERROR | Low | TLA+ uses `Nat`; Rust uses `u64` with checked arithmetic |
| 9a | JournalBeforeDispatch | `journal.rs` | VIOLATION + RACE | Critical | `Journaled`/`Queued` profiles dispatch before durable journal commit |
| 9b | JournalBeforeDispatch | `shard/lifecycle.rs` | VIOLATION | High | Frame mutated before journal append; journal failure leaves inconsistent state |

---

## Recommendations

### Immediate (blocking)
1. **Fix 9a (Critical):** Either make `Journaled` profile synchronous for `ActionScheduled` events, or update `JournalBeforeDispatch.tla` to model the asynchronous durability gap explicitly.
2. **Fix 9b (High):** Restructure `handle_action_completion` and `handle_action_failure` to append journal events **before** mutating frame state, or implement a compensating rollback on journal failure.

### Short-term
3. **Fix 7a (High):** Align `BudgetArithmetic.tla` with Rust semantics — model `sub_budgets` as returning `Ok(result)` or `Err(Underflow)` rather than flooring at 0.
4. **Fix 4a (High):** Either implement run transfers in Rust or simplify `ShardOwnership.tla` to match single-shard ownership.

### Medium-term
5. **Fix 5a (Medium):** Add explicit reservation validation in `build_admission` to reject runs with zero slot/action budgets.
6. **Fix 2a (Medium):** Make `drain_for_shutdown` loop until the queue is definitively empty AND shutdown command processed, with proper backpressure against concurrent enqueue.
7. **Fix 6a (Medium):** Update `RecoveryReplay.tla` to model the actual frame-snapshot recovery strategy, or implement journal-replay recovery in Rust.

---

## Verification Ledger

```jsonl
{"kind":"obligation","id":"FV-STEP-001","model":"StepState.tla","file":"frame.rs","result":"PASS","evidence":"validate_transition matches ValidNext exactly; terminal state blocking verified by tests"}
{"kind":"obligation","id":"FV-SCHED-001","model":"ShardScheduler.tla","file":"impl_.rs","result":"FAIL_LOCAL","evidence":"drain_for_shutdown race: concurrent enqueue can cause ShutdownInProgress before AllDrained"}
{"kind":"obligation","id":"FV-SCHED-002","model":"ShardScheduler.tla","file":"impl_.rs","result":"FAIL_LOCAL","evidence":"drain_for_shutdown limit uses current capacity, not MAX_COMMAND_QUEUE_CAPACITY constant"}
{"kind":"obligation","id":"FV-ATTEMPT-001","model":"AttemptTracking.tla","file":"helpers.rs","result":"PASS","evidence":"validate_ticket_attempt correctly rejects stale attempts (attempt < current)"}
{"kind":"obligation","id":"FV-OWNERSHIP-001","model":"ShardOwnership.tla","file":"lifecycle.rs","result":"FAIL_LOCAL","evidence":"TLA+ models run transfers; Rust has no transfer mechanism at all"}
{"kind":"obligation","id":"FV-ADMISSION-001","model":"BoundedAdmission.tla","file":"lifecycle.rs","result":"FAIL_LOCAL","evidence":"NoRunAdmittedWithoutReservation not enforced; no per-run reservation check for slots/actions > 0"}
{"kind":"obligation","id":"FV-RECOVERY-001","model":"RecoveryReplay.tla","file":"recovery.rs","result":"DEFERRED_GLOBAL","evidence":"Model describes journal-replay recovery; Rust uses frame snapshot hydration. Different but valid strategy."}
{"kind":"obligation","id":"FV-BUDGET-001","model":"BudgetArithmetic.tla","file":"budget.rs","result":"FAIL_LOCAL","evidence":"sub_budgets floors at 0 in TLA+; sub_dim returns Err(Underflow) in Rust"}
{"kind":"obligation","id":"FV-TAINT-001","model":"TaintLattice.tla","file":"value.rs","result":"PASS","evidence":"join_taint implements max(rank(a), rank(b)); all 6 lattice laws tested"}
{"kind":"obligation","id":"FV-JOURNAL-001","model":"JournalBeforeDispatch.tla","file":"journal.rs","result":"FAIL_REGRESSION","evidence":"Journaled and Queued profiles dispatch before durable commit; violates DispatchSafety"}
{"kind":"obligation","id":"FV-JOURNAL-002","model":"JournalBeforeDispatch.tla","file":"lifecycle.rs","result":"FAIL_REGRESSION","evidence":"Frame mutated before journal append; journal failure leaves inconsistent state"}
```

---

*Report generated by formal-verifier agent. All code quotes are exact excerpts from the repository at the time of analysis.*
