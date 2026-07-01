# Domain Model: vb-09aaz — Abort Batch on All Index Key Construction Failures

## Scope

This contract covers the G8 IndexKeyConstruction guard missing from
`JournalWriteBatch::append_event` at `crates/vb_storage/src/batch/append_event.rs:114-115`.
The defect is a partial-write hazard: the journal event is staged into
`self.inner` before the pending-action index mutation, but if
`stage_pending_action_index_op` returns `Err(KeyCapacity)` (the typed
error from `index_action_key`), the `?` propagates the error without
setting `self.aborted = true`. A subsequent caller-driven `commit()`
then persists the partially-staged batch — the event is durable but
the index update is not. The pending-action cursor is then
inconsistent with the durable event log, violating master §49
Crash-Consistency Rule.

This contract does not implement production Rust, write tests, or
write verifier artifacts. It models the G8 guard, the abort-on-error
invariant, and the abort-flag contract that is already canonical in
the `put_*` methods (`putters.rs`, 28 occurrences).

## Ubiquitous Language

- **Append event**: `JournalWriteBatch::append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError>` — stages a journal event into the batch's `OwnedWriteBatch` and updates the pending-action index in lock-step.
- **Pending action index maintenance**: the operation performed by `FjallJournal::stage_pending_action_index_op` (at `batch/action_index.rs:106-126`) that translates a `JournalEvent` into an insert/tombstone mutation on `self.journal.index_action` and stages it into the same `OwnedWriteBatch`.
- **Index key construction**: the call to `index_action_key(action, run, step)` inside `stage_pending_action_index_op`. Returns `Result<[u8; INDEX_ACTION_KEY_BYTES], JournalError>` where `INDEX_ACTION_KEY_BYTES = 13` (constants.rs:79).
- **Index key construction failure**: any `Err` returned by `stage_pending_action_index_op`. In production the reachable variant is `JournalError::KeyCapacity` (error/mod.rs:29, diagnostic code `KEY_CAPACITY_EXCEEDED` at codes.rs:103/196). The error is unreachable for nominal `ActionId/RunId/StepIdx` triples because the encoding (1 prefix + 2 + 8 + 2 bytes = 13) fits the fixed-length buffer exactly; the contract must still defend against it because the abort-on-fallible-step invariant is unconditional.
- **Abort flag (`aborted: bool`)**: the field on `JournalWriteBatch` (types.rs:26) set to `true` by any fallible step that, on `Err`, must prevent `commit()` from persisting the partial batch. Inspectable via `is_aborted()` (types.rs:67-70).
- **Abort-on-fallible-step contract**: the cross-method invariant that every fallible step in `JournalWriteBatch::append_event` and `put_*` methods sets `self.aborted = true` before propagating the typed error. `putters.rs` honors this in 28 distinct locations; `append_event` currently honors it only for `JournalError::DuplicateEvent` (the G3 durable-duplicate branch at append_event.rs:62).
- **Partially-staged batch**: a `JournalWriteBatch` whose `inner: OwnedWriteBatch` has at least one insert but whose abort flag is still `false`. Master §49 forbids `commit()` from persisting this state; the abort-on-fallible-step invariant is the mechanism that prevents it.
- **Crash-Consistency Rule (master §49)**: external side effects must not be dispatched until `ActionScheduled` is durably recorded under strict durability, and the durable record must include the pending-action index mutation atomically. A batch where the event is durable but the index is not violates §49 because recovery sees a run in flight but the index cursor is stale.

## Aggregate

`JournalWriteBatch<'j>` is the aggregate boundary for atomic write
admission. Its existing state is preserved by this contract; only
the abort-on-error policy for the G8 step is added.

Aggregate state already in place (types.rs:21-30):

1. `inner: fjall::OwnedWriteBatch` — staged storage operations.
2. `journal: &'j FjallJournal` — borrowed journal capability.
3. `staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]>` — same-batch duplicate guard.
4. `aborted: bool` — terminal-no-op flag.
5. `staged_bytes: u64` — accumulated encoded-byte total.
6. `byte_limit: Option<u64>` — batch byte budget.
7. `_not_send_or_sync: PhantomData<*mut FjallJournal>` — `!Send + !Sync` marker.

The fix at append_event.rs:114-115 changes only the post-condition
of the G8 step; it does not add fields, change types, or change the
public method signature. The signature
`pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError>`
remains unchanged.

## Entities and Value Objects

| Name | Kind | Contract |
| --- | --- | --- |
| `JournalWriteBatch<'j>` | Aggregate | Owns one atomic write session; `!Send + !Sync`; commits atomically or no-ops when aborted. |
| `JournalEvent` | Domain event | The runtime event being staged; relevant variants for G8 are `ActionScheduled`, `ActionScheduledTicket`, `ActionCompletedEvent`, `ActionFailedEvent`, `ActionCompletedEnvelope`, `ActionAbandoned` (action_index.rs:57-89). All other variants are no-ops for the index and therefore cannot trigger G8. |
| `PendingActionIndexOp` (internal) | Internal enum | `Insert { action, run, step }` or `Remove { action, run, step }`; the discriminated mutation staged by `stage_pending_action_index_op`. |
| `IndexActionKey` | Fixed-length byte array | `[u8; INDEX_ACTION_KEY_BYTES]` where `INDEX_ACTION_KEY_BYTES = 13`. The 13-byte layout is `[0x32][action_u16_be][run_u64_be][step_u16_be]` (constants.rs:43, keys.rs:139-155). |
| `JournalError::KeyCapacity` | Error variant | Returned by `index_action_key` on key-build overflow; diagnostic code `KEY_CAPACITY_EXCEEDED` (codes.rs:103, 196). Surfaced as the typed error from `stage_pending_action_index_op` to `append_event`. |
| `JournalError::BatchAborted` | Error variant | Returned by `commit()` when `self.aborted == true` (commit.rs:20-23). The terminal-no-op outcome. |
| `AppendEventOutcome` | Domain result | Either `Accepted { staged_bytes_after, inner_len_after, index_action_key_staged }` or `Rejected { reason, aborted: bool }` where `aborted` is the post-call state of the batch's abort flag. |

## Policies

1. **Abort-on-fallible-step**: every fallible step in `JournalWriteBatch::append_event` and `put_*` methods MUST set `self.aborted = true` before propagating a typed error. The pattern is the canonical `match ... { Ok(v) => v, Err(e) => { self.aborted = true; return Err(e); } }` already used in `putters.rs` lines 27-32, 33-39, 65-69, 70-76, 101-107, 132-138, 159-163, 164-170, 188-200, 212-223, 235-247. The G8 path (`stage_pending_action_index_op` at append_event.rs:114-115) MUST follow the same pattern.
2. **Guard Precedence (C6)**: the 8-guard order G1..G8 in `append_event` is: (G1) key construction `run_event_key`, (G2) same-batch duplicate, (G3) durable duplicate, (G4) batch count, (G5) per-record encoding, (G6) accumulated byte admission, (G7) inner.insert, (G8) `stage_pending_action_index_op`. G8 is the final fallible step before `staged_event_keys.insert(key)` and `Ok(())`. The doc-comment at append_event.rs:18-26 already lists 8 guards but only enumerates 7; G8 must be enumerated alongside G1..G7.
3. **Index-event reachability**: G8 is reachable only for events whose variant implies a pending-action-index mutation (action_index.rs:55-90). For events with no index implication, `stage_pending_action_index_op` returns `Ok(())` immediately and G8 cannot fire.
4. **Key-capacity reachability**: under production `index_action_key` (keys.rs:139-155), KeyCapacity is unreachable for nominal `ActionId(u16) × RunId(u64) × StepIdx(u16)` inputs because the encoding is exactly 13 bytes and the `ArrayVec<u8, 13>` buffer holds them with no slack. The contract treats KeyCapacity as DEFENSIVELY REACHABLE: the abort-on-fallible-step invariant is unconditional, even for fallible steps that are practically unreachable under normal operation.
5. **No partial persistence**: once `self.aborted == true`, the only legal state transitions for the batch are `commit() -> Err(BatchAborted)` (commit.rs:20-23) or drop. No subsequent `append_event` call may succeed; the precondition `!(*old(batch)).aborted` is preserved at the top of `append_event` (already in place at append_event.rs implicit precondition, formalized by the contract here).
6. **Idempotent same-batch rejection**: a subsequent `append_event` call with a key already in `staged_event_keys` (G2) does not change the abort flag. The G2 path returns `DuplicateStagedKey` without mutating `aborted`. This means a KeyCapacity failure at G8 followed by a subsequent same-key append will re-pass G1..G7 and only fail at the durable G3 check, since the event was never committed. This is documented as an open domain question (see Open Domain Decisions below) — a follow-up bead may move `staged_event_keys.insert(key)` before G8 to guarantee same-batch rejection.
7. **Typed-error propagation**: the typed error from G8 propagates to the caller. The caller inspects `batch.is_aborted()` and decides whether to retry on a fresh batch or surface the typed error to the operator.

## Commands

- `AppendJournalEvent(open_batch, event)` -> `Accepted` if all 8 guards pass, or `Rejected(reason)` where `reason` is one of the `JournalError` variants in the error taxonomy. On `KeyCapacity` rejection at G8, the batch is in the aborted state and `commit()` will return `Err(BatchAborted)`.
- `Commit(batch)` -> `Ok(())` on success, `Err(BatchAborted)` when `batch.aborted == true`. The `commit()` short-circuit at commit.rs:20-23 is the existing mechanism that honors the abort flag.

## Events

- `JournalEventStaged { key, staged_bytes_after, inner_len_after, index_action_key_staged }` after successful append.
- `JournalEventRejected { key, reason }` on any rejection path; `reason` is the `JournalError` variant.
- `BatchAborted { reason }` on `DuplicateEvent` (existing) or `KeyCapacity` (new at G8) — the only two paths that set `aborted = true` in `append_event`.

## Invariants

- I1: `JournalWriteBatch` is `!Send + !Sync` via `PhantomData<*mut FjallJournal>` (types.rs:18-21). The abort-on-fallible-step invariant is local to one batch handle; no cross-thread aliasing is possible.
- I2 (new): every fallible step in `append_event` that returns `Err` MUST have set `self.aborted = true` BEFORE returning, with the single exception of the G2 same-batch-duplicate path (`DuplicateStagedKey`) and the G4 count-capacity path (`QueueFull`) which are non-aborting rejections. The exceptions are pre-existing and unchanged.
- I3 (new): after `append_event` returns `Err(KeyCapacity)`, `batch.is_aborted() == true` and `batch.commit() == Err(BatchAborted)`.
- I4 (new): if `append_event` returns `Err(KeyCapacity)` at G8, the durable event log for the run is empty (no partial persistence). This follows from `commit()` short-circuiting on `aborted == true`.
- I5 (preserved): on the happy path, the event AND the pending-action-index mutation land in the same `OwnedWriteBatch`, so a single `commit()` makes them durable together.
- I6 (preserved): on the durable-duplicate path (G3), `aborted = true` is set, so any earlier valid staged events are NOT persisted. This is the existing behavior; the G8 fix brings G8 into parity with G3.
- I7 (preserved): the `commit()` short-circuit (commit.rs:20-23) returns `Err(BatchAborted)` whenever `aborted == true`. This is the consumer of the abort flag; the fix relies on it without modification.

## Forbidden States

- An `append_event` call returns `Err(KeyCapacity)` while `aborted == false` (the bug).
- An `append_event` call returns `Err(KeyCapacity)` and `commit()` then returns `Ok(())` (the partial-persistence variant).
- A `JournalWriteBatch` whose `inner: OwnedWriteBatch` has a non-zero `len()` AND whose `aborted == false` AND whose commit subsequently persists state without the corresponding index update. This is the master §49 violation.

## Open Domain Decisions

1. **Staged-event-keys insertion order**: at append_event.rs:119, the key is inserted into `staged_event_keys` AFTER the G8 `?`. If G8 fires, the key is not in the set, so a follow-up same-key append re-passes G1+G2 and only fails at G3 (durable lookup, which won't see the staged-but-uncommitted insert). This is an independent question. The contract RECOMMENDS moving `staged_event_keys.insert(key)` to immediately before the G8 call (after `inner.insert` succeeds) to guarantee same-batch rejection across G8-failed batches. This is flagged for the downstream contract owner.
2. **KeyCapacity reachability in spec**: the Verus production mirror at `verification/verus/production_inner/vb_vzcuf_PS_008_production.rs:151` and `_PS_009_production.rs:181` currently declares `KeyCapacity` as "unreachable in this mirror" because `key` is supplied as an exec arg. With G8 added, `KeyCapacity` becomes reachable from `stage_pending_action_index_op` via the index-key-construction step. The mirrors must be regenerated: either keep KeyCapacity unreachable for the run_event_key G1 path (key still supplied) and add a NEW guard entry for the index-key-construction step that returns `Err(KeyCapacity)`, or document KeyCapacity as a reachable variant from the G8 path. This decision belongs to the proof-writer; this contract only flags the requirement.
3. **Queued-writer behavior**: `JournalWriterQueue::stage_queued_event` (queue/writer/stage.rs:31-74) calls `stage_pending_action_index_op` at L72 with the same `?` propagation pattern. The batch is single-shot (`OwnedWriteBatch` is dropped, not committed), so there is no partial-write hazard. The contract explicitly does NOT require fixing the queued-writer path; that is a separate concern. If a follow-up bead wants to differentiate "permanent" vs "transient" failures at the queued-writer boundary, that is a separate contract.