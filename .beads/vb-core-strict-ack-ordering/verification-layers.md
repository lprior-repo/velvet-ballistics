# Verification Layers — vb-core-strict-ack-ordering

## Context

- **bead_id**: vb-core-strict-ack-ordering
- **bead_title**: runtime/storage: Prove strict persistence before acknowledgement ordering
- **phase**: State 3
- **updated_at**: 2026-05-15T00:00:00Z
- **attempt**: 1

## Boundary

| Layer | Scope | Owner |
|-------|-------|-------|
| TLA+ temporal model | Journal barrier state machine, EventSeq ordering, queued flush concurrency | This artifact (State 3) |
| Verus | Rust-local pure invariants, postconditions, type invariants for `DURABILITY_MATRIX`, `EventSeq`, `AckPoint`, `append_strict`/`append_journaled` | State 5/6 (proof-writer + proof-reviewer) |
| Kani | Bounded model check: `AckPoint` enum values in matrix, `append_strict` harness, codec roundtrips | State 11 (formal-verifier) |
| Loom | Concurrency: `JournalWriterQueue` flush ordering, concurrent submit with strict profile | State 11 (formal-verifier) |
| Miri | UB / provenance: `cfg(miri)` codec roundtrip tests | State 11 (formal-verifier) |
| Proptest | Journal event encoding/decoding roundtrips, EventSeq ordering invariants | State 8 (test-writer) |
| Integration tests | End-to-end: fail-closed on persist injection, restart recovery correctness | State 8 (test-writer) |
| Static scan | Clippy, `forbid(unsafe_code)` enforcement | State 11 (formal-verifier) |

---

## Layer Assignment per Contract Clause

### ACK-ORDER-001 / ACK-ORDER-002 (Strict Ack Ordering)

| Sub-clause | Primary Layer | Secondary | Evidence |
|-----------|--------------|-----------|---------|
| Every primitive uses `AfterJournalAppend` | **Verus** (`VERUS-DM-001`) | Kani harness | `verify_ack_after_persist` spec refinement |
| No row claims `BeforeJournalAppend` | **Verus** (`VERUS-DM-004`) | Kani bounded check | Runtime matrix enumeration |
| Ack only after persist for Strict | **Verus** (`VERUS-JA-001`) | TLA+ `I2` | `append_strict` postcondition + JournalBarrier model |
| Ack after journal for Journaled | **Verus** (`VERUS-JA-002`) | TLA+ `I3` | `append_journaled` postcondition + JournalBarrier model |

### INV-001 (Matrix Completeness)

| Sub-clause | Primary Layer | Evidence |
|-----------|--------------|---------|
| `DURABILITY_MATRIX.len() == REQUIRED_PRIMITIVES.len()` | **Verus** (`VERUS-DM-002`) | Completeness spec refinement |
| No duplicate primitives | **Verus** (`VERUS-DM-002`) | Deductive proof |
| Every `journal_events` non-empty | **Verus** (`VERUS-DM-002`) | Iterator check |

### INV-004 (EventSeq Ordering)

| Sub-clause | Primary Layer | Evidence |
|-----------|--------------|---------|
| `EventSeq::new(v).get() == v` | **Verus** (`VERUS-DM-003`) | Constructor injectivity proof |
| EventSeq serde roundtrip | **Proptest** + **Miri** | Roundtrip property test + miri run |
| Monotonic EventSeq advancement | **TLA+** (`EO1`-`EO3`) | EventSeqOrdering model |

### INV-006 (AckPoint Zombie Variant)

| Sub-clause | Primary Layer | Evidence |
|-----------|--------------|---------|
| `BeforeJournalAppend` unreachable via public API | **Kani** | Bounded harness proving no matrix row contains it |
| Compile-time removal consideration | **Verus** | Domain model review finding |

### DISPATCH-001 / DISPATCH-002 (Strict vs Journaled Dispatch)

| Sub-clause | Primary Layer | Evidence |
|-----------|--------------|---------|
| `Strict` profile calls `append_strict` | **Kani** | Dispatch harness |
| `Journaled` profile calls `append_journaled` | **Kani** | Dispatch harness |
| Queued strict flush uses barrier | **Loom** + **TLA+** (`QF1`-`QF3`) | QueuedStrictFlush model + loom concurrency tests |
| `flush_batch` calls `persist_strict` once per batch | **Loom** | Queue flush ordering test |

### FAIL-001 / FAIL-002 (Fail-Closed)

| Sub-clause | Primary Layer | Evidence |
|-----------|--------------|---------|
| Persist error returns typed error | **Integration test** | `storage_failure_before_header_prevents_ack` |
| No ack sent on persist failure | **Integration test** | `submit_direct_returns_durability_error_before_ack_when_header_cannot_persist` |
| `discard_journal_sequence` called on error | **Integration test** | Error path coverage |

### RECOVERY-001 / RECOVERY-002 / RECOVERY-003 (Restart)

| Sub-clause | Primary Layer | Evidence |
|-----------|--------------|---------|
| Restart state matches acknowledged state | **Integration test** + **Kani** | `restart_lookup_finds_persisted_header` + `hydrate_run_frame` harness |
| Digest matches on recovery | **Integration test** + **Kani** | `verify_digests` integration + Kani |
| Replay divergence detection | **Kani** | `ReplayDivergence` harness |

---

## Verus Scope

### Scope: `verify_ack_after_persist`

- **Rust target**: `crates/vb_runtime/src/durability_matrix.rs::verify_ack_after_persist`
- **Spec function**: `ack_point_is_after_append(row)` — checks `row.ack_point == AfterJournalAppend`
- **Proof surface**: `proof fn prove_verify_ack_after_persist()` — iterates static matrix, proves no `BeforeJournalAppend`
- **Invariants**: Static table completeness, no duplicate rows
- **Trusted boundary**: `DURABILITY_MATRIX` is a static compile-time table
- **Shell exclusions**: No I/O, async, storage, wall-clock time

### Scope: `append_strict` / `append_journaled`

- **Rust target**: `crates/vb_storage/src/journal/append.rs`
- **Spec functions**: `append_strict_spec`, `append_journaled_spec` — pure postcondition models
- **Proof surface**: Postconditions for each function
- **Trusted boundary**: `append_unpersisted` and `persist_strict` treated as oracle calls
- **Shell exclusions**: Only the Fjall oracle call is excluded; the composition logic is proven

---

## TLA+ Scope

### Scope: Journal Barrier State Machine

- **Module**: `JournalBarrier`
- **Variables**: `journaledEvents`, `persistedEvents`, `profile`, `ackSent`, `persistError`
- **Actions**: `AppendStrict`, `AppendJournaled`, `SendAck`, `PersistError`
- **Safety invariants**: `I1`-`I5`
- **Temporal properties**: `T1` (always ack-after-persist for Strict), `T2` (eventual ack liveness)
- **Fairness**: Weak fairness on `AppendStrict` and `SendAck`
- **Refinement boundary**: Rust `append_strict` refines `AppendStrict`; `append_journaled` refines `AppendJournaled`

### Scope: EventSeq Ordering

- **Module**: `EventSeqOrdering`
- **Variables**: `eventSeq`, `appendedSeqs`, `persistedSeqs`
- **Safety invariants**: `EO1`-`EO3`
- **Refinement boundary**: Runtime EventSeq advancement refines this model

### Scope: Queued Strict Flush

- **Module**: `QueuedStrictFlush`
- **Variables**: `queue`, `flushInProgress`, `strictFlushComplete`
- **Safety invariants**: `QF1`-`QF3`
- **Refinement boundary**: `QueuedStorageRuntimeJournal::flush_batch` refines `FlushBatch`

---

## Kani Scope

### Scope: `AckPoint` Matrix Values

- **Harness**: Enumerate all `DURABILITY_MATRIX` rows and assert each `row.ack_point == AfterJournalAppend`
- **Command**: `cargo kani --harness verify_ack_point_enum`
- **Expected**: No counterexample

### Scope: `append_strict` Dispatch

- **Harness**: Model `StorageRuntimeJournal::append_storage_event` with `Strict` profile, verify `persist_strict` is called before `Ok(())`
- **Command**: `cargo kani --harness verify_append_strict_barrier`
- **Expected**: No counterexample

### Scope: Codec Roundtrip

- **Harness**: All `RecordKind` values encode and decode correctly
- **Command**: `cargo kani --harness verify_record_kind_codec --features miri`
- **Expected**: No counterexample

---

## Loom Scope

### Scope: Queue Flush Ordering

- **Test**: `flush_batch_concurrent_submit` — concurrent submit with strict profile, verify barrier ordering
- **Test**: `shutdown_drain_strict` — drain queue on shutdown, verify all strict events persisted
- **Test**: `action_completion_cancel_during_flush` — cancel during strict flush, verify no partial ack
- **Command**: `cargo loom --test journal_queue_concurrency`

---

## Waiver Table

| Clause | Layer | Waiver Reason | Compensating Evidence |
|--------|-------|--------------|----------------------|
| Fjall `persist(SyncAll)` internal UB | N/A | External crate; treated as oracle | Kani harness on `persist_strict` boundary |
| Codec fuzzing beyond RecordKind | Proptest | RecordKind is bounded enum | `proptest` exhaustive variant test + Miri |
| Async runtime scheduling | N/A | No async strict ack path exists | `UnsupportedAsyncStrictAck` enforced + Kani |
