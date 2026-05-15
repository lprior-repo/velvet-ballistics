# TLA+ Temporal Model Plan: vb-qi37.4.2

## Boundary

- **TLA+-owned temporal behavior**: Journal write ordering, replay safety, action dispatch ordering, shard-level concurrency, lifecycle state transitions for steps, retry FSM, and capability lifecycle.
- **Rust/core behavior excluded from TLA+**: Taint lattice (Verus L4), StepBudget arithmetic (Verus L4), RunFrame dimension immutability (Verus L4), IPC/Record decode validation (Kani L3 + fuzz), expression evaluation (Kani/fuzz L2/L3), codegen parity (differential tests L1/L2).
- **External systems abstracted**: Fjall storage engine internals (compaction, WAL), OS scheduler, network transport.
- **Non-applicability rationale**: Not applicable for the taint lattice, finite-f64, numeric ID checked conversions, and budget arithmetic — these are pure Rust-local properties expressible in Verus. TLA+ is the correct tool for state-over-time behavior (journal ordering, replay, concurrency, lifecycle transitions).

---

## TLA+-Owned Clauses

### Journal Ordering (INV-013)

**Module**: `LifecycleJournal`
**Model path**: `verification/tla/LifecycleJournal.tla`
**Variables**:
- `journal: Seq(JournalEntry)` — ordered sequence of journal entries
- `dispatched: Set(ActionId)` — actions that have been dispatched
- `shardOwner: [ShardId -> MachineId]` — shard ownership map

**Invariants**:
- `JournalBeforeDispatch`: ∀e ∈ dispatched @ OrderIndex(journal, e) < OrderIndex(journal, ActionDispatch(e)) — every dispatched action has a prior journal entry
- `MonotonicSequence`: ∀i ∈ 1..Len(journal)-1 @ journal[i].seq < journal[i+1].seq — sequence numbers strictly increase

**Temporal Properties**:
- `EventuallyAllJournaled`: ∀a ∈ initiated @ □◇∃e ∈ journal @ e.action = a — every initiated action eventually appears in journal
- `NoOrphanDispatch`: dispatched ⊆ {e.action: e ∈ journal} — no action dispatched without a journal entry

**Fairness**: Weak fairness on WriteJournal and DispatchAction when enabled.
**Deadlock**: Journal state machine is deadlock-free by construction (append-only journal, no cycles).

**Refinement**: Rust `JournalWriter::write` appends entries in program order; the TLA+ `journal` sequence models this total order. Each shard's journal is modeled independently.

---

### Replay Safety (VB-REPLAY-001 to VB-REPLAY-007)

**Module**: `LifecycleJournal`, `ResumeStateMachine`
**Model path**: `verification/tla/LifecycleJournal.tla`, `verification/tla/ResumeStateMachine.tla`
**Variables** (extends above):
- `replayPointer: Nat` — current replay position in journal
- `replayed: Set(ActionId)` — actions already replayed
- `inFlight: Set(ActionId)` — actions dispatched but not yet completed

**Invariants**:
- `ReplayNoDuplicate`: ∀a ∈ replayed @ Count(journal, a) = 1 — each action appears exactly once in journal
- `ReplayOrderPreserved`: replayPointer always points to the oldest non-replayed entry
- `RecoveryIdempotent`: replaying the same journal twice produces identical state

**Temporal Properties**:
- `EventuallyReplayComplete`: ◇(replayed = initiated) — replay eventually catches up to all initiated actions
- `NoActionLostDuringReplay`: ∀a ∈ initiated @ a ∈ replayed ∨ a ∈ inFlight

**Evidence command**: `tlc -config verification/tla/LifecycleJournal.cfg verification/tla/LifecycleJournal.tla`
**Required cfg stance**: `LifecycleJournal.cfg` must check `PROPERTY EventuallyReplayComplete` and must not suppress deadlock checking unless a named stutter/fairness waiver is present.

---

### Concurrency / Shard Ownership (VB-CONC-001 to VB-CONC-005)

**Module**: `ConcurrencyControl`
**Model path**: `verification/tla/ConcurrencyControl.tla` (referenced by `VB-CONC` obligations)
**Variables**:
- `shards: [ShardId -> Procs]` — per-shard machine set
- `framePool: [ShardId -> Set(RunFrame)]` — frames per shard
- `globalLock: [ResourceId -> MachineId ∨ Nil]` — resource ownership

**Invariants**:
- `SingleShardOwner`: ∀f ∈ RunFrame @ ∃!s @ ShardOf(f) = s — each frame belongs to exactly one shard
- `NoCrossShardAlias`: ∀r @ globalLock[r] ≠ Nil ⇒ ownedBy(r) = callerShard
- `FramePoolBounded`: ∀s @ |framePool[s]| ≤ MAX_POOL_SIZE

**Temporal Properties**:
- `NoStarvation`: ∀shard @ □◇(framePool[shard] ≠ ∅ ⇒ eventuallyAllocated) — every non-empty pool eventually allocates
- `NoDeadlockOnLocks`: ◻◇(globalLock = [r → Nil]) — every lock is eventually released

**Fairness**: Weak fairness on AcquireFrame, ReleaseFrame, AcquireLock, ReleaseLock.
**Model bounds**: 3 shards, 5 frames per shard, 2 resources — small enough for exhaustive model checking.
**Required cfg stance**: `ConcurrencyControl.cfg` must check `PROPERTY NoDeadlockOnLocks`, `PROPERTY NoStarvation`, and `PROPERTY LockNoStarvation`; `CHECK_DEADLOCK FALSE` is not acceptable for State 6 approval unless the proof-reviewer records a clause-specific waiver.

---

### Retry FSM (VB-REPLAY-004, VB-REPLAY-005)

**Module**: `RetryFSM`
**Model path**: `verification/tla/RetryFSM.tla`
**Variables**:
- `retryState: [ActionId -> {Idle, Attempting, Backoff, Done, Exhausted}]`
- `attemptCount: [ActionId -> Nat]`
- `backoffUntil: [ActionId -> Nat]`

**Invariants**:
- `MaxAttemptsRespected`: attemptCount[a] ≤ max_retries_per_action
- `BackoffDurationPositive`: backoffUntil[a] > currentTime ⇒ state = Backoff

**Temporal Properties**:
- `EventuallyExhaustedOrDone`: ∀a @ □(retryState[a] ∈ {Done, Exhausted}) — every action eventually terminates
**Required cfg stance**: `RetryFSM.cfg` must check `PROPERTY EventuallyExhaustedOrDone`; deadlock suppression requires a named waiver.

---

### Capability Lifecycle (VB-REPLAY-006, VB-REPLAY-007)

**Module**: `CapabilityLifecycle`
**Model path**: `verification/tla/CapabilityLifecycle.tla`
**Variables**:
- `capabilities: Set(CapabilityId)`
- `held: [MachineId -> Set(CapabilityId)]`

**Invariants**:
- `CapabilityUniqueOwner`: ∀c ∈ capabilities @ |{m: c ∈ held[m]}| = 1 — each capability held by exactly one machine
- `ValidCapabilityAccess`: access(m, c) ⇒ c ∈ held[m]

---

## Existing TLA+ Specs (referenced, not modified)

| Spec | Path | Owner |
|------|------|-------|
| LifecycleJournal | `verification/tla/LifecycleJournal.tla` | vb-qi37.4.2 |
| RetryJournal | `verification/tla/RetryJournal.tla` | vb-qi37.4.2 |
| RetryFSM | `verification/tla/RetryFSM.tla` | vb-qi37.4.2 |
| AskAnswerLifecycle | `verification/tla/AskAnswerLifecycle.tla` | vb-qi37.4.2 |
| ResumeStateMachine | `verification/tla/ResumeStateMachine.tla` | vb-qi37.4.2 |
| CapabilityLifecycle | `verification/tla/CapabilityLifecycle.tla` | vb-qi37.4.2 |

---

## Evidence Commands

```bash
# Journal ordering + replay safety
tlc -config verification/tla/LifecycleJournal.cfg verification/tla/LifecycleJournal.tla

# Retry FSM
tlc -config verification/tla/RetryFSM.cfg verification/tla/RetryFSM.tla

# Concurrency control
tlc -config verification/tla/ConcurrencyControl.cfg verification/tla/ConcurrencyControl.tla

# Optional: Apalache symbolic check for liveness
apalache-mc check --config=verification/tla/LifecycleJournal.cfg verification/tla/LifecycleJournal.tla
```

---

## Waivers

- **WAIVER-TLA-01**: clauses `INV-001` to `INV-006`; waived layer `tla-plus`; reason: taint lattice laws are pure Rust-local algebraic properties with no temporal/state-over-time behavior; limitation: does not waive Verus/proptest/Kani evidence; compensating evidence: `verus verification/verus/taint_lattice.rs`, `kani_taint_propagation`, and `taint_property_join`; owner: `rust-contract/proof-planner`; expiry/follow-up: expires if taint propagation becomes workflow/concurrent state.
- **WAIVER-TLA-02**: clause `INV-008`; waived layer `tla-plus`; reason: StepBudget monotonicity is a pure arithmetic burn-down invariant; limitation: does not waive Verus/Kani evidence for underflow and monotonicity; compensating evidence: `verus verification/verus/step_budget.rs` and `kani_step_budget`; owner: `rust-contract/proof-planner`; expiry/follow-up: expires if budget mutation becomes concurrent/shared temporal state.
- **WAIVER-TLA-03**: clause `PRE-003`; waived layer `tla-plus`; reason: FiniteF64 finiteness is a constructor validation property; limitation: does not waive finite-value unit/property evidence; compensating evidence: `finite_f64_property` and constructor tests; owner: `rust-contract/test-planner`; expiry/follow-up: expires if finite-f64 participates in a temporal protocol.
- **WAIVER-TLA-04**: clauses `INV-011`, `INV-012`; waived layer `tla-plus`; reason: reject-before-allocation is pure parser/control-flow behavior; limitation: does not waive Kani/fuzz reject-before-allocation evidence; compensating evidence: `kani_ipc_header`, `kani_record_*`, `ipc_decode`, and `record_decode`; owner: `rust-contract/formal-verifier`; expiry/follow-up: expires if decoding becomes stateful protocol negotiation.
- **WAIVER-TLA-05**: clause `INV-009`; waived layer `tla-plus`; reason: numeric ID index safety is local bounds/access behavior; limitation: does not waive Kani and forbidden-scan obligations; compensating evidence: `kani_index_access` and `cargo xtask forbidden-scan --pattern as_usize_index --crate vb_core`; owner: `rust-contract/formal-verifier`; expiry/follow-up: expires if index allocation becomes a temporal lifecycle.
