# Workflow Model — vb-hn4sc

bead_id: vb-hn4sc
phase: 3 (rust-contract)
isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-hn4sc
captured_at: 2026-07-01T15:32:00Z
authoring_agent: rust-contract

This artifact specifies the legal states, transitions, guards, outcomes, terminal states, and retry semantics of `JournalWriterQueue::flush_batch` under the new byte-budget gate. State machine is explicit so proof and implementation can lock onto the same shape.

## 1. Entities and Their Lifecycle States

### 1.1 `JournalWriterQueue` aggregate

| State | Predicate | Entry Transition | Exit Transition |
|---|---|---|---|
| `Fresh` | `pending.is_empty() && !shutdown` | `new()` constructor returns `Ok` | first `enqueue_*` |
| `Accepting` | `!shutdown && pending.len() < capacity` | `enqueue_*` returns `Ok` | `pending.len() == capacity` OR `shutdown` |
| `AtCapacity` | `!shutdown && pending.len() == capacity` | `enqueue_*` returns `Ok` from `Accepting` | `flush_batch` drains one or more OR `shutdown` |
| `Flushing` (transient) | lock held; `pending` partially drained into `OwnedWriteBatch`; commit in progress | `flush_batch` acquires lock | `flush_batch` returns |
| `ShuttingDown` | `shutdown == true` | `shutdown()` sets the flag | `drain_all` returns |
| `Closed` | `shutdown == true && pending.is_empty()` | `drain_all` returns with `Ok(report)` | (terminal) |

The byte-budget gate is a *transition guard* on `FlushStart -> StageNext`, not a state of its own. It either lets the event be staged or rejects the entire `flush_batch` call.

### 1.2 Per-event sub-state inside `flush_batch`

| Sub-state | Predicate | Transition |
|---|---|---|
| `PreScan` | `batch_len = 0; has_strict = false; accumulated_bytes = 0` | `flush_batch` start |
| `ScanProfile` | walking `pending[0..min(pending.len(), batch_size)]` to compute `has_strict` and `batch_len` | `PreScan` -> `StageEvents` |
| `StageEvents` | for each `pending[0..batch_len)`, call `stage_queued_event`; track `accumulated_bytes`; the byte gate fires **after** the `staged_keys` dedup check **and** before `owned_batch.insert` | `ScanProfile` -> `GateDecision` |
| `GateAccept` | `accumulated_bytes + next_event_encoded_len <= limit` | `StageEvents` -> `Staged` (advance) |
| `GateReject` | `accumulated_bytes + next_event_encoded_len > limit` | `StageEvents` -> `Abort` (return `Err`) |
| `GateOverflow` | `accumulated_bytes.checked_add(next_event_encoded_len).is_none()` | `StageEvents` -> `Abort` (return `Err` with `attempted = u64::MAX`) |
| `Staged` | event inserted into `OwnedWriteBatch` | `GateAccept` -> next event |
| `Abort` | return `Err(JournalBatchBytesExceeded)`; **do not call `owned_batch.commit()`** | `GateReject` / `GateOverflow` -> `flush_batch` returns `Err` |
| `Commit` | `accumulated_bytes <= limit` for every staged event; `owned_batch.commit()` invoked | `Staged` loop finishes -> `DrainPending` |
| `DrainPending` | `pop_front` exactly `written` items from `state.pending` | `Commit` -> `Ok(report)` |
| `Ok` | return `Ok(JournalWriterFlushReport { drained, written })` | `DrainPending` -> `flush_batch` returns `Ok` |

## 2. Transition Diagram (textual)

```
        +--------+   enqueue_*   +-----------+   pending.len()==cap  +-----------+
        | Fresh  |------------->| Accepting |--------------------->| AtCapacity |
        +--------+              +-----------+                       +-----------+
            ^                         |                                  |
            |                         | flush_batch                      |
            |                         v                                  v
            |                  +---------------+       ScanProfile
            |                  |   PreScan     |--------------------+
            |                  +---------------+                    v
            |                                                  +--------------+
            |                                                  | StageEvents  |
            |                                                  +--------------+
            |                                                  |     |     |
            |                                       next fits  |     |     | next oversize
            |                                       within     |     |     | limit
            |                                       budget     v     |     v
            |                                     +-----------+     |  +--------+
            |                                     | GateAccept|---->|  | Gate   |
            |                                     +-----------+     |  | Reject |
            |                                           |           |  +--------+
            |                                           |           |     |
            |                                           |           |     v
            |                                           |           |  +--------+
            |                                           |           |  | Abort  |--Err-->
            |                                           |           |  +--------+
            |                                           |           |
            |                                           |     checked_add
            |                                           |     overflow
            |                                           |           |
            |                                           |           v
            |                                           |     +--------+
            |                                           |     | Gate   |
            |                                           |     |Overflow|
            |                                           |     +--------+
            |                                           |        |Err
            |                                           v        v
            |                                     +--------+-------+
            |                                     |   Staged       |
            |                                     +----------------+
            |                                           |
            |                                           v
            |                                     +--------+
            |                                     | Commit |--owns batch.commit
            |                                     +--------+
            |                                           |
            |                                           v
            |                                     +---------------+
            |                                     | DrainPending  |
            |                                     +---------------+
            |                                           |
            |                                           v
            |                                     +--------+
            |                                     |   Ok   |
            |                                     +--------+
            |
            +--------------drain_all returns with empty pending
```

## 3. Guards

| Guard | Predicate | Where it fires | On failure |
|---|---|---|---|
| `lock_acquired` | `Mutex::lock().is_ok()` | every `flush_batch` start | `JournalError::WriteLockPoisoned` |
| `not_shutdown` | `state.shutdown == false` | only `enqueue_*` (NOT `flush_batch` itself, since `flush_batch` is a drain operation that must run after shutdown) | `JournalError::QueueShutdown` |
| `within_capacity` | `state.pending.len() < self.capacity` | `enqueue_*` | `JournalError::QueueFull` |
| `batch_within_batch_size` | `batch_len < self.batch_size` | `ScanProfile` loop | loop terminates |
| `has_pending` | `batch_len > 0` | after `ScanProfile` | `Ok(report { drained: 0, written: 0 })` |
| `staged_keys_unique` | `staged_keys.insert(key).is_none()` (inside `stage_queued_event`) | per-event inside `StageEvents` | `JournalError::DuplicateStagedKey` |
| `durable_key_unique` | `journal.events.contains_key(key) == false` (inside `stage_queued_event`) | per-event inside `StageEvents` | `JournalError::DuplicateEvent`; batch is aborted (existing behavior; this is the *direct* path's behavior; the queued path inherits) |
| `payload_within_event_cap` | `value.len() <= MAX_JOURNAL_EVENT_PAYLOAD_BYTES` | inside `encode_record` | `JournalError::PayloadTooLarge` |
| `event_valid` | `event.is_valid()` | inside `stage_queued_event` | `JournalError::InvalidEvent` |
| **`byte_gate_within_batch`** (NEW) | `accumulated_bytes + next_event_encoded_len <= self.byte_budget` | per-event inside `StageEvents`, AFTER `staged_keys_unique` and AFTER `durable_key_unique`, BEFORE `owned_batch.insert` | `JournalError::JournalBatchBytesExceeded { attempted, limit }` |
| `commit_succeeds` | `owned_batch.commit().is_ok()` | after `Commit` | `JournalError::Fjall(_)` (propagated) |
| `drain_count_matches_staged` | `state.pending.pop_front()` returns `Some(_)` exactly `written` times | inside `DrainPending` | `JournalError::WriteLockPoisoned` (LOGIC INVARIANT) |

## 4. Guard Precedence (C6 contract, queued variant)

The order in `flush_batch` per-event is:

1. Lock acquisition.
2. `ScanProfile` (profile flag walk, `batch_len` count).
3. Per event, in order:
   1. `staged_keys_unique` (HashSet dedup).
   2. `durable_key_unique` (Fjall lookup; aborts batch on hit).
   3. `encode_record` (builds the encoded value; rejects `PayloadTooLarge`).
   4. **`byte_gate_within_batch` (NEW).**
   5. `owned_batch.insert`.
   6. `staged_event_keys.insert` (post-staging, to mark for subsequent dup checks).
   7. `stage_pending_action_index_op` (atomic index maintenance).
4. `Commit` (SyncAll iff `has_strict`).
5. `DrainPending` (exactly `written` pop_fronts).

The byte gate at step 3.4 is the only addition. It runs *after* duplicate detection (steps 3.1 and 3.2) so `flush_batch_rejects_same_batch_duplicate_key` continues to pass.

## 5. Outcomes

### 5.1 `Ok(JournalWriterFlushReport { drained: usize, written: usize })`

- Conditions: lock acquired; `ScanProfile` walked; every staged event passed all per-event guards including the new byte gate; `owned_batch.commit()` succeeded; `DrainPending` popped exactly `written` items.
- Invariants: `drained == written > 0` OR `drained == written == 0` (the empty-queue early return).
- Side effects: Fjall holds `written` new records; queue holds `pending.len() - drained` fewer events.

### 5.2 `Err(JournalError::JournalBatchBytesExceeded { attempted, limit })` (NEW terminal outcome)

- Conditions: lock acquired; `ScanProfile` completed; some number `k < batch_len` events passed `staged_keys_unique`, `durable_key_unique`, `encode_record`, and `byte_gate_within_batch`; the `(k+1)`-th event's encoded length `e` is such that either:
  - `accumulated_bytes + e > limit`, OR
  - `accumulated_bytes.checked_add(e).is_none()` (overflow → `attempted = u64::MAX`).
- Side effects:
  - `owned_batch.commit()` is **NOT** called.
  - The first `k` events stay in `state.pending` (NOT drained).
  - `JournalError::JournalBatchBytesExceeded` is returned.
- `attempted` value:
  - On overflow: `u64::MAX`.
  - Otherwise: `accumulated_bytes + e` (the sum that *would* have been committed if the limit did not exist).
- `limit` value: `self.byte_budget` (the active limit, identical to `StorageLimits::max_journal_batch_bytes` for the queue's configuration).
- Caller-visible behavior: the queue still holds the rejected event plus all not-yet-staged events. A subsequent `flush_batch` call will re-attempt the byte gate with the same first event (still rejected until the offending event is dropped, dequeued by other means, or the budget is raised).

### 5.3 `Err(JournalError)` from other guards

`WriteLockPoisoned`, `DuplicateStagedKey`, `DuplicateEvent`, `Encode`, `PayloadTooLarge`, `InvalidEvent`, `Fjall`, `QueueCapacity`. These are all pre-existing and unchanged.

## 6. `drain_all` and `shutdown` Compositional Behavior

`drain_all` iterates `flush_batch` up to `ceil(capacity / batch_size) + 2` times.

- On the *first* byte-budget rejection, `drain_all` returns `Err(JournalBatchBytesExceeded)` immediately (no further flushes attempted).
- This is the same pattern as `DuplicateStagedKey` and `DuplicateEvent`: any error from `flush_batch` short-circuits the drain.
- `shutdown` flips the `shutdown` flag (only `enqueue_*` cares about this flag; `flush_batch` continues to drain even after shutdown). The drain's terminal outcome mirrors `drain_all`.

## 7. Retries and Idempotency

- **Retry of `flush_batch` after a byte-budget rejection:** safe at the queue-state level (no partial commit happened), but the same event will be rejected again unless the caller drops it, raises the budget, or splits the payload.
- **Retry of `drain_all` after a byte-budget rejection:** also safe; the same first event will reject again.
- **Cross-flush idempotency:** unaffected. `stage_queued_event` still checks `journal.events.contains_key(key)` for committed events; the byte gate does not change this.
- **Crash recovery:** unaffected. Master §49 atomicity holds because `owned_batch.commit()` is the only durability boundary and the gate prevents its invocation on a violating batch.

## 8. Cancellation

`JournalWriterQueue` does not currently support cancellation tokens. There is no `async` path on the queue itself; the mutex is `std::sync::Mutex`. A `flush_batch` call cannot be cancelled mid-flight except by poisoning the mutex, which surfaces as `WriteLockPoisoned`.

The byte-budget gate does NOT introduce new cancellation paths.

## 9. Terminal States Summary

| Terminal state | Reachable via | Queue state after | Fjall state after |
|---|---|---|---|
| `Ok(report)` | successful `flush_batch` | `pending.len()` reduced by `drained` | `written` new records |
| `Err(JournalBatchBytesExceeded)` | byte gate rejection | `pending.len()` unchanged | unchanged |
| `Err(DuplicateStagedKey)` | duplicate `(run, seq)` in same flush | `pending.len()` unchanged | unchanged |
| `Err(DuplicateEvent)` | committed duplicate | `pending.len()` unchanged | unchanged |
| `Err(WriteLockPoisoned)` | mutex poisoning | `pending.len()` unchanged | unchanged |
| `Err(Fjall)` | commit failure | `pending.len()` unchanged | unchanged (commit was atomic) |
| `Err(Encode/PayloadTooLarge/InvalidEvent)` | malformed event | `pending.len()` unchanged | unchanged |