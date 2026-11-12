# TLA+ Temporal Model Plan: vb-c1s0

## Boundary

### Temporal/Workflow Behavior (TLA+-Owned)
- Multi-shard command routing determinism (`RunId % shard_count`)
- Per-shard FIFO command processing (one command per tick)
- Run lifecycle state transitions: Admitted → Running ↔ AwaitingAction/AwaitingAsk/AwaitingTimer → Terminal
- Timer wheel firing: deadline ordering and generation-based authority
- Action completion routing and run resumption
- Ask/answer routing and run resumption
- Graceful shutdown sequence

### Rust/Core Behavior Excluded from TLA+ (Verus/Kani/tests)
- Timer generation arithmetic monotonicity (Verus pure fn)
- BoundedActionCompletionQueue capacity enforcement (Verus + Miri)
- Step budget exhaustion correctness (Verus pure fn)
- RunFrame step state machine internal invariants (Verus)
- CompiledWorkflow node validity (Verus)

### External Systems Abstracted
- CLI user interaction
- Journal storage backend (SharedRuntimeJournal)
- Wall-clock time source (modeled as discrete ticks)
- External action completion callbacks
- External ask answer callbacks

### Non-Applicability Rationale
Not applicable: **FALSE** — This bead has significant temporal/state-over-time behavior requiring TLA+:
1. Multi-shard routing determinism
2. Command processing order per tick
3. Run lifecycle state machine
4. Timer wheel firing semantics
5. Concurrency-safe action queue operations

---

## TLA+-Owned Clauses

### TLA-WF-001: RunId Shard Routing Determinism
- **Contract clause**: INV-001
- **Module**: `MultiShardRuntime`
- **Property**: `shard_for(run)` returns the same shard for any `run` at all times

### TLA-WF-002: Per-Shard FIFO Command Processing
- **Contract clause**: INV-007, POST-005
- **Module**: `ShardProcessing`
- **Property**: Commands dequeued in same order enqueued; at most one command per tick

### TLA-WF-003: Run Terminal State Uniqueness
- **Contract clause**: POST-002
- **Module**: `RunLifecycle`
- **Property**: Each run reaches exactly one terminal state (Finished, Failed, Cancelled)

### TLA-WF-004: Timer Authority Handoff
- **Contract clause**: POST-004, INV-003
- **Module**: `TimerWheel`
- **Property**: `timer_entry_fired` only fires matching entries; stale entries ignored

### TLA-WF-005: Action Completion Routing
- **Contract clause**: POST-003
- **Module**: `ActionRouting`
- **Property**: `complete_action_with_output` delivers to correct RunId and resumes at correct step

### TLA-WF-006: Run Lifecycle Liveness
- **Contract clause**: implicit liveness
- **Module**: `RunLifecycle`
- **Property**: Every non-terminal run eventually reaches a terminal state, awaits external input, or continues indefinitely

---

## Model Shape

### Module: `MultiShardRuntime`

```
VARIABLES
  shards,          \* Seq(ShardState)
  shard_count,    \* Nat
  run_to_shard    \* RunId -> Nat

Init ==
  /\ shard_count \in Nat \ {0}
  /\ shards = [i \in 1..shard_count |-> ShardInit(i)]
  /\ run_to_shard = [run \in Runs |-> (run.id mod shard_count) + 1]

Submit(run, workflow) ==
  LET shard == shards[run_to_shard[run]]
  IN
    /\ shard.admission_valid
    /\ shards' = [shards EXCEPT ![run_to_shard[run]] = SubmitToShard(shard, run, workflow)]
    /\ UNCHANGED run_to_shard
```

### Module: `ShardProcessing`

```
VARIABLES
  command_queues,  \* Seq(Vec(ShardCommand))
  processing        \* Nat (current shard being ticked)

TickAll ==
  /\ \A i \in DOMAIN command_queues:
       Len(command_queues[i]) > 0 =>
         ProcessOneCommand(shards[i], Head(command_queues[i]))
  /\ \A i \in DOMAIN command_queues:
       command_queues[i]' = IF Len(command_queues[i]) > 0
                            THEN Tail(command_queues[i])
                            ELSE command_queues[i]
```

### Module: `TimerWheel`

```
VARIABLES
  by_deadline,  \* BTreeMap Instant -> Set TimerEntry
  by_run,       \* [RunId -> TimerEntry]

InsertTimer(run, deadline, kind) ==
  LET gen == IF run \in DOMAIN by_run
             THEN by_run[run].generation + 1
             ELSE 1
  IN
    /\ by_run' = by_run @@ (run :> [run |-> run, generation |-> gen, deadline |-> deadline, kind |-> kind])
    /\ by_deadline' = AddToDeadlineIndex(by_deadline, deadline, by_run'[run])

FireExpired(now) ==
  LET expired == {e \in UNION Range(by_deadline): e.deadline <= now}
  IN
    /\ \A e \in expired: e \in Range(by_run)
    /\ by_run' = [r \in (DOMAIN by_run) \ {e.run: e \in expired} |-> by_run[r]]
    /\ by_deadline' = [d \in DOMAIN by_deadline |-> by_deadline[d] \ {e \in expired: e.deadline = d}]
```

### Module: `RunLifecycle`

```
VARIABLES
  runs,           \* [RunId -> RunState]
  run_status      \* [RunId -> Status]

Status == {Admitted, Running, AwaitingAction, AwaitingAsk, AwaitingTimer, Finished, Failed, Cancelled}

TerminalStatus == {Finished, Failed, Cancelled}

InitStatus(run) ==
  IF run \in DOMAIN runs THEN runs[run].status ELSE Nil

TerminalInvariant ==
  \A run \in DOMAIN run_status:
    run_status[run] \in TerminalStatus =>
      \A cmd \in Commands: \* no commands processed for terminal run
        ~IsCommandForRun(cmd, run)

TerminalReached ==
  \A run \in DOMAIN run_status:
    run_status[run] \in TerminalStatus
```

---

## Properties

### Safety Invariants

| ID | Name | Formula |
|----|------|---------|
| SI-001 | NoDoubleRouting | `run_to_shard[run] = run_to_shard[run]` (idempotent) |
| SI-002 | QueueFIFO | `dequeue_order = enqueue_order` per shard |
| SI-003 | TerminalUniqueness | `\A run: |\A s \in TerminalStatus: run_status[run] = s| <= 1` |
| SI-004 | TimerGenerationMonotonic | `gen2 > gen1` for same run on successive inserts |
| SI-005 | NoPhantomTimerFire | `timer_entry_fired` matches current timer state |
| SI-006 | QueueCapacityBounded | `queue_len <= capacity` at all times |

### Liveness/Temporal Properties

| ID | Name | Formula |
|----|------|---------|
| LT-001 | EventuallyTerminal | `<>(\A run: run_status[run] \in TerminalStatus)` for all runs |
| LT-002 | EventuallyResumed | `[](\A run: run_status[run] = AwaitingAction) => <>(run_status[run] # AwaitingAction)` |
| LT-003 | EventuallyProgress | `[](\A run: run_status[run] = Running) => <>(\E s: run_status[run] # s)` |
| LT-004 | NoInfiniteAwait | `<>(\A run: run_status[run] \in {AwaitingAction, AwaitingAsk, AwaitingTimer} => <>(run_status[run] \notin {AwaitingAction, AwaitingAsk, AwaitingTimer}))` |

### Fairness Assumptions

- **Weak Fairness**: `TickAll` action is weakly fair (shard eventually makes progress if always enabled)
- **Strong Fairness**: `Submit` action is strongly fair (submitted runs eventually begin processing)
- **No Fairness Assumption**: External action/ask/timer completion (by definition, not controlled by runtime)

### Deadlock Freedom

```
DeadlockFree ==
  \A shards, command_queues:
    \E shard \in DOMAIN shards:
      Len(command_queues[shard]) > 0
    \/ \E run \in DOMAIN runs:
      run_status[run] \in {Running, AwaitingAction, AwaitingAsk, AwaitingTimer}
    \/ \E shard \in DOMAIN shards:
      shards[shard].status # ShuttingDown
```

---

## State Constraints (for TLC)

| Constraint | Rationale |
|------------|-----------|
| `shard_count <= 4` | Bounded model for model checking |
| `MaxRuns <= 8` | Limit RunId space |
| `MaxTimersPerShard <= 4` | Bounded timer wheel |
| `MaxQueueDepth <= 3` | Bounded command queue |
| `MaxStepsPerRun <= 5` | Bounded workflow execution |

---

## Symmetry Sets

| Set | Symmetry |
|-----|----------|
| `RunIds` | Exhaustively enumerated: `1..MaxRuns` |
| `ShardIndices` | Exhaustively enumerated: `1..shard_count` |
| `TimerKinds` | `{Wait, Ask}` |

---

## Bounded Model Limits

- `MaxRuns = 8` (RunId 1..8)
- `MaxShardCount = 4` (configurable)
- `MaxQueueDepth = 3` (command queue depth per shard)
- `MaxTimers = 4` (pending timers per shard)
- `MaxSteps = 5` (steps per workflow)

---

## Refinement to Rust/Runtime Behavior

| TLA+ Variable | Rust Type | Mapping |
|---------------|-----------|---------|
| `shards[i].command_queue` | `ArrayQueue<ShardCommand>` | TLC Seq models FIFO order |
| `shards[i].runs[run].status` | `RunState.status` | Status enum variants |
| `shards[i].timer_wheel.by_run[run]` | `TimerEntry` | Generation, deadline, kind |
| `run_to_shard[run]` | `runtime.shard_for(run)` | `run.get() % shard_count` |

**Refinement Invariant**: For every concrete `RunId` and `shard_count`, `shard_for(run)` in Rust must return `shards[(run.get() % shard_count)]`.

---

## Evidence Command

```bash
# TLC model checking
tlc -config specs/MultiShardRuntime.cfg specs/MultiShardRuntime.tla
tlc -config specs/ShardProcessing.cfg specs/ShardProcessing.tla
tlc -config specs/TimerWheel.cfg specs/TimerWheel.tla
tlc -config specs/RunLifecycle.cfg specs/RunLifecycle.tla

# Apalache symbolic checking (alternative)
apalache-mc check --config=specs/MultiShardRuntime.tla.cfg specs/MultiShardRuntime.tla
```

---

## Waivers

| Clause | Owner | Reason | Expiry | Compensating Evidence |
|--------|-------|--------|--------|----------------------|
| TLA-WF-006 (Liveness) | priorlewis43 | External action/ask completion not modeled | Ongoing | BDD scenario coverage + integration tests |

---

## Open Questions

- **DISCOVERY_BLOCKED**: Whether the TLA+ model should include journal event emission
- **DISCOVERY_BLOCKED**: Whether `SubmitPrePersisted` command variant affects routing semantics
