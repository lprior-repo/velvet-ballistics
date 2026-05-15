# TLA+ Temporal Model Plan — vb-core-strict-ack-ordering

## Context

- **bead_id**: vb-core-strict-ack-ordering
- **bead_title**: runtime/storage: Prove strict persistence before acknowledgement ordering
- **phase**: State 3
- **updated_at**: 2026-05-15T00:00:00Z
- **attempt**: 1

## Boundary

### What IS in TLA+

- **Temporal behavior of journal append + persist barrier**: The strict journal path must atomically append-then-persist before returning ack.
- **EventSeq ordering**: Events for the same run must be appended in monotonically increasing EventSeq order.
- **Strict vs Journaled dispatch**: The `DurabilityProfile` selector determines whether a barrier is required.
- **Concurrency of queued journal writes**: Multiple `Strict`-profile events in the `JournalWriterQueue` must each receive a barrier on `flush_batch`.
- **Fail-closed error propagation**: If persist fails, ack must not be sent.

### What is NOT in TLA+

- **Rust type-level invariants**: `AckPoint` enum validity, `NonZeroUsize` constructors, `EventSeq` monotonicity proofs — these are Verus/Kani targets.
- **Codec/serde roundtrips**: Covered by proptest + Miri.
- **Fjall internal fsync implementation**: Treated as an oracle — we assume `persist(SyncAll)` achieves durable storage.
- **Recovery/replay logic**: Covered by integration tests + Kani.
- **Network/distributed coordination**: Not applicable to this bead.

---

## TLA+-Owned Clauses

### ACK-ORDER-TLA-001: Strict Journal Barrier Safety

**Contract clause**: `ACK-ORDER-001`

**Module**: `JournalBarrier`

**Variables**:
- `journaledEvents: Seq(RecordKind)` — append-only event log for the current run
- `persistedEvents: Seq(RecordKind)` — subset of `journaledEvents` confirmed durable
- `profile: {"Strict", "Journaled", "Volatile"}` — durability profile
- `ackSent: BOOLEAN` — whether acknowledgement has been returned to caller
- `persistError: BOOLEAN` — whether persist barrier failed

**Init**:
```
journaledEvents = <<>>
persistedEvents = <<>>
ackSent = FALSE
persistError = FALSE
```

**AppendStrict(event)**:
```
IF profile = "Strict" THEN
    journaledEvents' = Append(journaledEvents, event)
    persistedEvents' = Append(persistedEvents, event)  \* persist barrier: event lands in persisted atomically
    ackSent' = FALSE  \* no ack until after barrier confirmed
ELSIF profile = "Journaled" THEN
    journaledEvents' = Append(journaledEvents, event)
    \* persistedEvents unchanged — no barrier
    ackSent' = FALSE
ELSE
    \* Volatile: no journal write
    UNCHANGED <<journaledEvents, persistedEvents>>
END IF
```

**SendAck**:
```
IF profile = "Strict" THEN
    ackSent' = (persistedEvents = journaledEvents)  \* all events persisted before ack
ELSIF profile = "Journaled" THEN
    ackSent' = (journaledEvents # <<>>)  \* any event appended is enough (group commit)
ELSE
    ackSent' = FALSE
END IF
```

**PersistError**:
```
persistError' = TRUE
ackSent' = FALSE
```

**Safety Invariants**:

- `I1`: `persistedEvents` is a prefix of `journaledEvents` (never persist what hasn't been appended)
- `I2`: `ackSent = TRUE` implies `persistedEvents = journaledEvents` for `Strict` profile
- `I3`: `ackSent = TRUE` implies `journaledEvents # <<>>` for `Journaled` profile
- `I4`: `persistError = TRUE` implies `ackSent = FALSE` (fail-closed)
- `I5`: Events appear in `journaledEvents` in strictly increasing EventSeq order (modeled externally via EventSeq action)

**Temporal Properties**:

- `T1`: `[]I2` — Always-true safety for strict ack ordering
- `T2`: `<>(ackSent = TRUE)` — Every run that appends at least one event eventually gets ack (liveness, under fairness)

**Fairness**:
- Weak fairness on `AppendStrict` and `PersistError` under their respective guard conditions
- Weak fairness on `SendAck` when its guard condition holds

---

### ACK-ORDER-TLA-002: EventSeq Strict Ordering

**Contract clause**: `POST-009`, `INV-004`

**Module**: `EventSeqOrdering`

**Variables**:
- `eventSeq: Nat` — current EventSeq counter
- `appendedSeqs: Set(Nat)` — EventSeq values that have been appended in this run
- `persistedSeqs: Set(Nat)` — EventSeq values that have been persisted

**AppendEvent(seq)**:
```
IF seq > eventSeq THEN
    eventSeq' = seq
    appendedSeqs' = appendedSeqs ∪ {seq}
ELSIF seq = eventSeq + 1 THEN
    eventSeq' = seq
    appendedSeqs' = appendedSeqs ∪ {seq}
ELSE
    \* seq ≤ eventSeq: violates strict ordering
    eventSeq' = eventSeq
END IF
```

**PersistEvent(seq)**:
```
IF seq ∈ appendedSeqs THEN
    persistedSeqs' = persistedSeqs ∪ {seq}
END IF
```

**Safety Invariants**:

- `EO1`: `persistedSeqs ⊆ appendedSeqs` — never persist what wasn't appended
- `EO2`: `∀seq ∈ appendedSeqs: seq ≤ eventSeq` — appended seqs never exceed counter
- `EO3`: `appendedSeqs` contains only consecutive values from 0 up to `eventSeq` (no gaps in normal operation)

---

### ACK-ORDER-TLA-003: Queued Strict Flush Ordering

**Contract clause**: `DISPATCH-002`

**Module**: `QueuedStrictFlush`

**Variables**:
- `queue: Seq(RecordKind)` — pending events in the queue
- `flushInProgress: BOOLEAN` — whether a flush is currently executing
- `strictFlushComplete: BOOLEAN` — whether all strict events have been flushed with barrier

**EnqueueStrict(event)**:
```
queue' = Append(queue, event)
flushInProgress' = FALSE
strictFlushComplete' = FALSE
```

**FlushBatch**:
```
IF queue # <<>> THEN
    \* Each event in queue receives individual append_strict + shared persist_strict
    \* Model as: all events appended to journal, then single persist barrier
    flushInProgress' = TRUE
    strictFlushComplete' = TRUE  \* barrier confirmed
ELSE
    flushInProgress' = FALSE
    strictFlushComplete' = TRUE
END IF
```

**Safety Invariants**:
- `QF1`: `strictFlushComplete = TRUE` implies all queued events were appended
- `QF2`: `flushInProgress = TRUE` implies no new events can be enqueued (atomic flush)
- `QF3`: For `Strict` profile events, `persist_strict` is called exactly once per `FlushBatch`, not per event

---

## Evidence Commands

### For ACK-ORDER-TLA-001

```bash
# TLC model check for JournalBarrier
tlc -config specs/JournalBarrier.cfg specs/JournalBarrier.tla
```

Expected: No invariant violations on `I1` through `I5`. Temporal property `T1` holds. `T2` liveness verified under fairness.

### For ACK-ORDER-TLA-002

```bash
# TLC model check for EventSeqOrdering
tlc -config specs/EventSeqOrdering.cfg specs/EventSeqOrdering.tla
```

Expected: No invariant violations on `EO1` through `EO3`.

### For ACK-ORDER-TLA-003

```bash
# TLC model check for QueuedStrictFlush
tlc -config specs/QueuedStrictFlush.cfg specs/QueuedStrictFlush.tla
```

Expected: No invariant violations on `QF1` through `QF3`.

---

## Non-Applicability Rationale

The TLA+ specs above model the **runtime journal semantics** — append, barrier, ack ordering — as a state machine. This is the correct abstraction level for TLA+ because:

1. The runtime's `append_strict` / `append_journaled` dispatch is a pure state transition
2. EventSeq ordering is enforced by the runtime's sequence advancement logic
3. The queued strict flush is a concurrency protocol

The Rust types (`EventSeq`, `AckPoint`, `DurabilityProfile`) are **not** modeled in TLA+ — their invariants are proven in Verus/Kani. The TLA+ model treats `EventSeq` as a natural number and `append_strict` as an atomic append-then-persist action.

---

## Waivers

| Waiver | Clause | Owner | Reason | Compensating Evidence |
|--------|--------|-------|--------|-----------------------|
| W1 | TLA+ for Fjall fsync internals | rust-contract | Fjall is an external crate; its fsync implementation is treated as oracle | Kani harness on `persist_strict` + integration tests |
| W2 | TLA+ for codec roundtrips | rust-contract | Serde roundtrip is not a temporal protocol; covered by proptest + Miri | `proptest` event encoding tests + `cargo miri` runs |
| W3 | TLA+ for recovery/replay | rust-contract | Recovery is tested via integration test suite; not a concurrent protocol | Kani harness on `hydrate_run_frame` + integration tests |
