---
section: 68
title: "Durable Execution Architecture Contract"
parent: velvet-ballistics-MASTER.md
---

## 68. Durable Execution Architecture Contract


> **Target contract.** The invariants in this section describe the intended architecture. Current implementation has frame-seed hydration for recovered slot values, taint, and step states, but live pending-action hydration and strict async acknowledgement paths remain gated. Summary-only recovery still returns `UnsupportedFullRecoveryHydration`, and `UnsupportedAsyncStrictAck` remains in the code until strict durability acknowledgement evidence is complete.

`velvet-ballistics` is a log-first durable execution engine. The architecture follows the same core model as production-grade orchestrators (AWS Step Functions): journal events are the ground truth, state is deterministically derived from the journal, and side effects are never re-executed without explicit idempotency proof.

### Log-First Invariants

1. **Journal entry persisted = step happened.** Once a journal event is durably written (according to the active durability profile), that step is committed. Recovery never re-executes it without idempotency proof.
2. **State is derived from journal, never the reverse.** Slot values, taint arrays, step states, and run status are all reconstructed by replaying journal events. No mutable state is the source of truth.
3. **Side effects are never re-executed during replay unless declared idempotent.** Non-idempotent actions are blocked during replay by `ActionReplayTracker`. Idempotent actions require matching `ActionTicket.idempotency_key` on re-execution.
4. **Recovery is deterministic.** Replaying the same journal events on the same compiled workflow digest must produce identical slot values, taint, step states, and terminal result. Any divergence is a `ReplayDiverged` error.
5. **Journal sequence numbers are monotonic per run.** No gaps, no reordering. `SeqNo` is `u64` and wraps are forbidden (typed error before wrap).

### Recovery Model

Recovery follows the snapshot-plus-tail pattern:

1. Load latest snapshot for the run (slot values, taint, step states at sequence N).
2. Replay journal events from sequence N+1 onward.
3. Each event is applied deterministically: `SlotWritten` updates slot+taint, `StepStarted`/`StepSucceeded` advances state machine, `ActionScheduled`/`ActionCompleted`/`ActionFailed` track action lifecycle.
4. Terminal events (`RunFinished`, `RunFailed`, `RunCancelled`) end replay.
5. If any event cannot be applied (missing prerequisite state, digest mismatch, corrupt record), recovery fails with a typed error — never silently continues.

Epoch-based recovery (future): Crash recovery should support a "seal and start new segment" model where the current journal segment is sealed on crash detection and a new segment begins, preventing partial-write ambiguity. This is not required for v1 single-server but the journal format must not preclude it.

### Single-Server Contract

`velvet-ballistics` is a single-server engine. There is no distributed replication, no leader election, no quorum consensus, and no control plane. These are explicit v1 exclusions:

- No Raft/Paxos consensus.
- No multi-node replication.
- No partition rebalancing.
- No distributed log (Bifrost-equivalent).
- No disaggregated storage tiering to object stores.

The single-server constraint means:
- Fjall is the sole durability mechanism. If the node loses power, recovery depends on Fjall's write-ahead log surviving the crash.
- Strict durability mode (`persist_strict` + `fsync`) is the only profile that guarantees no data loss on power failure.
- Journaled mode provides bounded data loss window (group commit batch interval).
- Volatile mode is testing-only and accepts full loss on crash.

### Tiered Durability Model

| Profile | Write Path | Crash Safety | Use Case |
|---------|-----------|--------------|----------|
| `volatile` | No Fjall writes | None — all data lost | Benchmarks, unit tests |
| `journaled` | Bounded Fjall writer queue, group commit | Bounded loss window (last batch) | Production default |
| `strict` | Synchronous Fjall persist + fsync before ack | Zero data loss | Financial, compliance |

### Compilation vs Interpretation

Unlike orchestrators that interpret journal entries against SDK code (opaque foreign processes), the current `velvet-ballistics` milestone compiles workflows to numeric IR and executes that IR through the interpreter:

| Mode | Execution | When to Use |
|------|-----------|-------------|
| IR interpreter | Dispatch through `CompiledNodeKind` enum | Current backend execution, debugging, portability, replay validation |

Generated Rust is removed from the current execution model. IR interpreter is the only accepted execution mode.

### Bounded Execution Contract

Every execution dimension is bounded by `ResourceContract`. The engine must reject or suspend before exceeding any bound. Silent truncation is forbidden.

Key bounds enforced at runtime:
- Steps per tick (`StepBudget`)
- Total slots, expressions, constants, accessors (compile-time)
- Expression stack depth (evaluator)
- Queue depth (shard command queue)
- Journal batch bytes (writer queue)
- Fanout branches, collect items, retry attempts (per-primitive)
- ValueStore arena entries (per-run cap, Phase 45)

This is the Holzmann influence: bounded loops, bounded allocation, no hidden growth vectors.

### Taint Propagation

`Taint` is a three-level lattice: `Clean < DerivedFromSecret < Secret`. Propagation rules:

1. `EvalExpr` joins taint from all loaded input slots.
2. `BuildObject`/`BuildList` join taint from all field/item slots.
3. `Finish` carries taint in the result signal `(SlotValue, Taint)`.
4. Validation does not reject tainted finish results; `Clean`, `DerivedFromSecret`, and `Secret` finish taints are preserved in the result signal.
5. Action output taint must be at least as restrictive as input taint for `DeterministicPure` and `IdempotentExternal` actions.
6. `AtLeastOnceExternal` actions propagate conservatively as `DerivedFromSecret` when any input is tainted.
7. Secret-tainted failure details must not enter public diagnostics without redaction.

---
