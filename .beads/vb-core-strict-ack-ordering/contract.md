# Contract Specification — vb-core-strict-ack-ordering

## Context

- **bead_id**: vb-core-strict-ack-ordering
- **bead_title**: runtime/storage: Prove strict persistence before acknowledgement ordering
- **phase**: State 3 (Contract and type model)
- **updated_at**: 2026-05-15T00:00:00Z
- **attempt**: 1

## Domain Terms

| Term | Definition |
|------|------------|
| `AckPoint` | Enum with `AfterJournalAppend` (correct) and `BeforeJournalAppend` (forbidden) |
| `AckPoint::AfterJournalAppend` | Shard returns `Ok(())` only after journal append + persist barrier confirmed |
| `AckPoint::BeforeJournalAppend` | Acknowledgement before persistence — zero tolerance, never allowed |
| `DURABILITY_MATRIX` | Immutable static table mapping each YAML primitive to its `AckPoint`, journal events, storage partition |
| `verify_ack_after_persist()` | Pure function that iterates `DURABILITY_MATRIX` and fails if any row has `BeforeJournalAppend` |
| `verify_matrix_completeness()` | Pure function asserting all 11 `REQUIRED_PRIMITIVES` have a matrix row |
| `verify_matrix_replay_proofs()` | Pure function asserting every row has at least one `test_evidence` path |
| `verify_matrix()` | Composes all three matrix verifiers; returns `Ok(())` only when all pass |
| `append_strict` | FjallJournal: `append_unpersisted(event)?; persist_strict()?; Ok(())` — full barrier |
| `append_journaled` | FjallJournal: `append_unpersisted(event)?; Ok(())` — no barrier, group commit only |
| `persist_strict` | FjallJournal: `database.persist(fjall::PersistMode::SyncAll)?; Ok(())` |
| `DurabilityProfile::Strict` | Runtime config: selects `append_strict` path |
| `DurabilityProfile::Journaled` | Runtime config: selects `append_journaled` path |
| `DurabilityProfile::Volatile` | Runtime config: no Fjall writes during run |
| `EventSeq` | Monotonic per-run u64 sequencer; `ZERO` = 0, `MIN` = 0, `MAX` = u64::MAX |
| `StoragePartition` | `RuntimeJournal`, `ActionJournal`, `TimerJournal` — keyspace selector |

## Required Primitives

`set`, `do`, `choose`, `for_each`, `together`, `collect`, `reduce`, `repeat`, `wait`, `ask`, `finish`

## Assumptions

- Fjall `persist(fjall::PersistMode::SyncAll)` provides hardware-level durability (OS fsync equivalent)
- No async strict ack path exists at the runtime boundary (`UnsupportedAsyncStrictAck` error variant is enforced)
- All 11 primitives are covered by `DURABILITY_MATRIX`; no new primitives will be added without contract update
- `QueuedStorageRuntimeJournal::flush_batch` preserves strict ordering: all queued strict events are flushed before `Ok(())` is returned
- `StorageRuntimeJournal::append_storage_event` dispatches to `append_strict` when `DurabilityProfile::Strict` is active

## Open Questions

- **OQ-1**: Does `QueuedStorageRuntimeJournal::flush_batch` guarantee the same barrier semantics as `append_strict` when `Strict` profile is queued? Need Kani + Loom confirmation.
- **OQ-2**: Is `BeforeJournalAppend` variant of `AckPoint` reachable through any public API, or is it compile-time unreachable? Kani harness should confirm zero construction paths.
- **OQ-3**: Are all 11 `test_evidence` paths in `DURABILITY_MATRIX` actually implemented tests, or are they stubs? Integration test audit required.

---

## Preconditions

- **PRE-001**: `DURABILITY_MATRIX` is a non-empty slice of `DurabilityRow` with exactly 11 rows (one per `REQUIRED_PRIMITIVES` entry).
- **PRE-002**: `REQUIRED_PRIMITIVES` contains no duplicate entries.
- **PRE-003**: Every `DurabilityRow.primitive` in `DURABILITY_MATRIX` is a member of `REQUIRED_PRIMITIVES`.
- **PRE-004**: `DurabilityRow.journal_events` is non-empty for every row.
- **PRE-005**: `DurabilityRow.primitive` and `DurabilityRow.compiled_node_kind` are non-empty static strings.
- **PRE-006**: `EventSeq::new(v)` produces a sequencer for any `u64` value `v` without panicking.
- **PRE-007**: `JournalQueueCapacity::new(nz)` and `JournalBatchSize::new(nz)` accept only `NonZeroUsize` values.

---

## Postconditions

- **POST-001**: `verify_ack_after_persist()` returns `Ok(())` iff every `DurabilityRow.ack_point` in `DURABILITY_MATRIX` equals `AckPoint::AfterJournalAppend`.
- **POST-002**: `verify_ack_after_persist()` returns `Err(DurabilityError::AckBeforePersist { primitive, handler })` where `primitive` and `handler` match the violating row when any row claims `BeforeJournalAppend`.
- **POST-003**: `verify_matrix_completeness()` returns `Ok(())` iff `DURABILITY_MATRIX` has a row for every entry in `REQUIRED_PRIMITIVES`.
- **POST-004**: `verify_matrix_replay_proofs()` returns `Ok(())` iff every `DurabilityRow.test_evidence` slice is non-empty.
- **POST-005**: `verify_matrix()` returns `Ok(())` iff all three subordinate verifiers pass.
- **POST-006**: `append_strict` returns `Ok(())` only after both `append_unpersisted` and `persist_strict` succeed.
- **POST-007**: `append_journaled` returns `Ok(())` immediately after `append_unpersisted` succeeds, with no durability barrier.
- **POST-008**: `persist_strict` returns `Ok(())` only after `database.persist(fjall::PersistMode::SyncAll)` succeeds.
- **POST-009**: `EventSeq::get(EventSeq::new(v)) == v` for all `u64` values `v`.
- **POST-010**: `EventSeq::new(v).get() >= EventSeq::ZERO.get()` for all `v` (non-negative).

---

## Invariants

- **INV-001**: `DURABILITY_MATRIX.len() == REQUIRED_PRIMITIVES.len()` — one row per required primitive, no extras.
- **INV-002**: No `DurabilityRow` in `DURABILITY_MATRIX` has `ack_point == AckPoint::BeforeJournalAppend`.
- **INV-003**: All `RecordKind` values used in `journal_events` arrays are valid discriminants (1-50 inclusive).
- **INV-004**: `EventSeq` ordering is preserved across serde round-trips: `EventSeq::new(v).serialize().deserialize() == EventSeq::new(v)`.
- **INV-005**: `AckPoint` enum has exactly two variants: `AfterJournalAppend` and `BeforeJournalAppend`.
- **INV-006**: `AckPoint::BeforeJournalAppend` is unreachable through any public constructor; attempting to construct it via public API returns a compile-time error or is blocked by `verify_ack_after_persist`.
- **INV-007**: `DurabilityProfile` enum has exactly three variants: `Volatile`, `Journaled`, `Strict`.
- **INV-008**: The `Strict` profile path (`append_strict`) always calls `persist_strict` before returning `Ok(())`.
- **INV-009**: The `Journaled` profile path (`append_journaled`) never calls `persist_strict` during normal operation.
- **INV-010**: `JournalWriterQueueProfileCounts` correctly tallies `journaled` vs `strict` pending writes at any snapshot.

---

## Error Taxonomy

All journal/storage errors are typed and exhaustive:

| Variant | Trigger | Semantics |
|---------|---------|-----------|
| `JournalError::QueueCapacity` | `JournalQueueCapacity::try_from_usize(0)` | Queue capacity must be non-zero |
| `JournalError::QueueCapacity` | `JournalBatchSize::try_from_usize(0)` | Batch size must be non-zero |
| `JournalError::DigestMismatch` | BLAKE3 digest mismatch on event read-back | Corrupt or tampered journal record |
| `JournalError::SeqenceGapped` | EventSeq gap detected during replay | Missing events between acknowledged seqs |
| `JournalError::ProcessLockTaken` | Double-open attempt on same Fjall process | Process-level lock prevents double-open |
| `RuntimeError::StorageJournalAppend` | `append_strict` / `append_journaled` failure | Typed wrapper around `JournalError` |
| `RuntimeError::AdmissionHeaderPersistenceFailed` | Run header persist fails before ack | Typed wrapper around `JournalError` |
| `RuntimeError::UnsupportedAsyncStrictAck` | Strict ack requested in async context | Async strict ack not supported |
| `RuntimeError::EncodeFailed` | Event encoding fails | BLAKE3 digest or serde error |
| `RuntimeError::QueueFull` | Journal probe fails | Queue health check rejects |

---

## Contract Signatures

```rust
// vb_runtime/src/durability_matrix.rs
pub fn verify_ack_after_persist() -> Result<(), DurabilityError>
pub fn verify_matrix_completeness() -> Result<(), DurabilityError>
pub fn verify_matrix_replay_proofs() -> Result<(), DurabilityError>
pub fn verify_matrix() -> Result<(), DurabilityError>

// vb_storage/src/journal/append.rs
pub fn append_strict(&self, event: &JournalEvent) -> Result<(), JournalError>
pub fn append_journaled(&self, event: &JournalEvent) -> Result<(), JournalError>
pub fn persist_strict(&self) -> Result<(), JournalError>

// vb_storage/src/types.rs
pub const fn new(value: u64) -> Self  // EventSeq
pub const fn get(self) -> u64          // EventSeq
```

---

## AckPoint Ordering Contract

This is the central contract of the bead:

> **ACK-ORDER-001**: For every primitive in `DURABILITY_MATRIX`, the shard acknowledgement returned to the caller MUST NOT occur until after the corresponding journal events have been durably persisted via a `persist_strict` barrier.

Equivalently:

> **ACK-ORDER-002**: `verify_ack_after_persist() == Ok(())` is a mandatory pre-condition for accepting the `DURABILITY_MATRIX` as correct. Any row claiming `BeforeJournalAppend` makes the entire matrix invalid.

### Strict vs Journaled Policies

| Property | `Strict` | `Journaled` | `Volatile` |
|----------|---------|-------------|------------|
| Persistence barrier | `persist_strict` per event | None (group commit) | None |
| `ack_point` | `AfterJournalAppend` | `AfterJournalAppend` (by matrix row) | Not in matrix |
| Zero data loss | Yes | Bounded group-commit window | No durability |
| Async strict ack | Blocked (`UnsupportedAsyncStrictAck`) | N/A | N/A |

---

## Strict vs Journaled Dispatch Contract

> **DISPATCH-001**: `StorageRuntimeJournal::append_storage_event` MUST dispatch to `append_strict` when `DurabilityProfile::Strict` is active, and MUST dispatch to `append_journaled` when `DurabilityProfile::Journaled` is active.

> **DISPATCH-002**: `QueuedStorageRuntimeJournal::flush_batch` MUST call `append_strict` (not `append_journaled`) for every queued event when `DurabilityProfile::Strict` is active, and MUST call `persist_strict` exactly once after all strict events are appended.

---

## Fail-Closed Contract

> **FAIL-001**: Any `append_strict`, `append_journaled`, or `persist_strict` failure MUST propagate as a typed `RuntimeError::StorageJournalAppend` or `RuntimeError::AdmissionHeaderPersistenceFailed` to the caller, and the run MUST NOT be acknowledged.

> **FAIL-002**: The journal error path MUST call `discard_journal_sequence` to prevent partial state from being committed.

---

## Restart / Recovery Contract

> **RECOVERY-001**: Restart recovery MUST produce an acknowledged state that exactly matches the state that would exist if the run had been replayed from journal events.

> **RECOVERY-002**: `hydrate_run_frame_from_events` MUST produce a `RecoveryFrameSeed` whose digest matches the persisted header digest.

> **RECOVERY-003**: `replay_events` MUST detect and fail on `ReplayDivergence` when step ordering diverges from the acknowledged log.
