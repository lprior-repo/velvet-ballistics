# Workflow Model: vb-09aaz — Abort Batch on All Index Key Construction Failures

## Typestates

```text
Unbuilt
  -> Open { aborted = false, staged_bytes = 0, len = 0 }
Open
  -> Open { aborted' = aborted } on G1/G2/G4/G5/G6/G7 rejection (non-aborting)
Open
  -> Aborted on G3 DurableDuplicate rejection (DuplicateEvent, existing behavior)
Open
  -> Aborted on G8 IndexKeyConstruction rejection (KeyCapacity, NEW — fix)
Open
  -> Open { aborted = false, len += 1, staged_event_keys += key } on success
Open
  -> Committed on successful commit (aborted = false required)
Aborted
  -> TerminalNoOp on commit (returns Err(BatchAborted))
Committed
  -> Terminal
```

The new state transition introduced by this contract is:

```
Open --(G8 IndexKeyConstruction Err(KeyCapacity))--> Aborted
```

## Append Event Decision Table

| Step | Guard | Accepted transition | Rejected outcome | Mutates bytes? | Aborts? |
| --- | --- | --- | --- | --- | --- |
| 1 | G1 `run_event_key` builds | continue | `JournalError::KeyCapacity` | no | no |
| 2 | `event.is_valid()` is true | continue | `JournalError::InvalidEvent` | no | no |
| 3 | G2 `staged_event_keys` lacks key | continue | `JournalError::DuplicateStagedKey` | no | no |
| 4 | G3 `journal.events.contains_key(key)` is false | continue | `JournalError::DuplicateEvent` | no | **YES** (existing behavior, append_event.rs:62) |
| 5 | G4 `inner.len() < MAX_BATCH_COUNT` | continue | `JournalError::QueueFull` | no | no |
| 6 | G5 `encode_record(...)` Ok | continue | `JournalError::Encode` / `PayloadTooLarge` | no | no |
| 7 | G6 `byte_limit.checked_add(encoded_len)` Ok AND `attempted <= limit` | `staged_bytes := attempted` | `JournalError::JournalBatchBytesExceeded` / `SequenceOverflow` | yes on accept | no |
| 8 | G7 `inner.insert(events, key, value)` | `inner_len += 1` | (infallible side-effect) | yes | no |
| 9 | **G8 `stage_pending_action_index_op` Ok** | `Ok(())` | **`JournalError::KeyCapacity`** | no | **YES (NEW — fix)** |
| 10 | post-G8 `staged_event_keys.insert(key)` | `staged_event_keys += key` | (infallible side-effect) | yes | no |

After step 10 the function returns `Ok(())`. Step 9 is the new
abort-on-error site introduced by the fix. The fix is
LOCAL to step 9: replace the `?` with a `map_err` that sets
`self.aborted = true` and returns the typed error, matching the
canonical pattern in `putters.rs:188-200, 212-223, 235-247`.

## Index-Event Reachability

G8 is reachable ONLY for `JournalEvent` variants whose action-
lifecycle class implies a pending-action-index mutation:

| Variant | Index mutation | G8 path |
| --- | --- | --- |
| `ActionScheduled { action, run, step, ... }` | Insert | YES |
| `ActionScheduledTicket { ticket, ... }` | Insert | YES |
| `ActionCompletedEvent { action, run, step, ... }` | Remove | YES |
| `ActionFailedEvent { action, run, step, ... }` | Remove | YES |
| `ActionCompletedEnvelope { ticket, ... }` | Remove | YES |
| `ActionAbandoned { ticket, ... }` | Remove | YES |
| `RunAccepted`, `StepStarted`, `StepEnded`, `SlotWritten`, `WaitScheduled`, `AskScheduled`, `WaitResolved`, `AskAnswered`, `RunFinished`, `RunFailed`, `RunCancelled`, etc. | None | NO (returns Ok immediately) |

For variants with no index implication, `stage_pending_action_index_op`
returns `Ok(())` immediately (action_index.rs:111-113), so G8 cannot
fire. The abort-on-fallible-step invariant still applies
trivially — the Ok path does not set `aborted`.

For index-implying variants, G8 fires `index_action_key(action, run, step)?`
(action_index.rs:116, 121). On `Err(KeyCapacity)`, the function
returns immediately (action_index.rs:111-125), and the typed error
propagates up to `append_event` at append_event.rs:114-115.

## KeyCapacity Reachability

Under production `keys::index_action_key` (keys.rs:139-155), the
KeyCapacity error is unreachable for nominal inputs:

- `INDEX_ACTION_KEY_BYTES = 13` (constants.rs:79).
- Layout: `[0x32 prefix][action u16 be][run u64 be][step u16 be]` = 1 + 2 + 8 + 2 = 13 bytes.
- The `ArrayVec<u8, 13>` buffer holds 13 bytes exactly with no slack.
- `try_push(PREFIX_INDEX_ACTION)` (1 byte) — buffer was 0/13.
- `try_extend_from_slice(action.to_be_bytes())` (2 bytes) — buffer was 1/13, becomes 3/13.
- `try_extend_from_slice(run.to_be_bytes())` (8 bytes) — buffer was 3/13, becomes 11/13.
- `try_extend_from_slice(step.to_be_bytes())` (2 bytes) — buffer was 11/13, becomes 13/13.
- `into_inner()` — succeeds because the buffer is at capacity.

For `ActionId::new(value)` where `value <= u16::MAX = 65535`,
`RunId::new(value)` where `value <= u64::MAX`,
`StepIdx::new(value)` where `value <= u16::MAX = 65535`,
the encoding always fits in 13 bytes. KeyCapacity is therefore
DEFENSIVELY REACHABLE in the contract: the abort-on-fallible-step
invariant is unconditional, even for fallible steps that are
practically unreachable.

The contract does not require changing `index_action_key` to make
KeyCapacity reachable; the production call site is already correct.
The contract requires that IF KeyCapacity were ever returned from
`stage_pending_action_index_op`, the batch would be aborted and
no partial persistence would occur.

## Commit Workflow

- If `Open` (`aborted == false`), commit writes all staged operations including the journal event AND the index-action mutation.
- If `Aborted` (`aborted == true`), commit returns `Err(JournalError::BatchAborted)` without writing (commit.rs:20-23).
- The `commit()` short-circuit is the existing mechanism that enforces the abort invariant.

## Same-Batch Duplicate Workflow

The current behavior:

- `staged_event_keys.insert(key)` happens at append_event.rs:119,
  AFTER the G8 `?`.
- If G8 fires (KeyCapacity), the key is NOT in `staged_event_keys`.
- A subsequent `append_event` call with the same `(run, seq)` will:
    - Pass G1 (run_event_key succeeds).
    - Pass G2 (key not in `staged_event_keys`).
    - Hit G3 (durable duplicate check via `journal.events.contains_key(key)?`). Since the original event was never committed (the batch aborted), `journal.events.contains_key` returns `false`. The batch passes G3.
    - Hit G4-G7 with fresh state.
    - Hit G8 again. If G8 still fails (KeyCapacity), the batch aborts again with the same error.
    - If G8 succeeds (e.g. the original G8 failure was transient), the batch proceeds normally and commits.

This is a subtle but well-defined behavior. The contract recommends
moving `staged_event_keys.insert(key)` to before G8 (after the G7
inner.insert) to guarantee same-batch rejection across G8-failed
batches. This is an open domain decision flagged in
`domain-model.md`. The current behavior is acceptable for the
vb-09aaz fix; the reorder is a separate optimization.

## Terminal Outcomes

- `Accepted`: all 8 guards pass; event staged; index mutation staged; `staged_event_keys` updated; commit succeeds.
- `RejectedNonAborting`: G1, G2, G4, G5, or G6 rejection; previous valid batch remains committable; `aborted == false` preserved.
- `RejectedAborting` (existing G3): durable duplicate; `aborted = true`; subsequent `commit()` returns `Err(BatchAborted)`.
- `RejectedAborting` (NEW G8): index-key construction failure; `aborted = true`; subsequent `commit()` returns `Err(BatchAborted)`.
- `Committed`: durable Fjall batch committed.
- `TerminalNoOp`: aborted batch commit no-ops.

## Concurrency / Async

- `JournalWriteBatch` is `!Send + !Sync` via `PhantomData<*mut FjallJournal>` (types.rs:18-21). The abort-on-fallible-step invariant is local to one batch handle; no cross-thread aliasing is possible. No loom or scheduling proof is required.
- The queued-writer path (`JournalWriterQueue::flush_batch` at queue/writer.rs:152-231 and `stage_queued_event` at queue/writer/stage.rs:31-74) is single-shot: the `OwnedWriteBatch` is owned and dropped on `Err`, never committed. No partial-write hazard at the queued-writer site. The contract does not require fixing that path.

## Callers and Re-Entry

- A caller that receives `Err(KeyCapacity)` from `append_event` may inspect `batch.is_aborted()` to confirm the batch is aborted.
- A caller that receives `Err(KeyCapacity)` may attempt to retry on a fresh `JournalWriteBatch`. The original aborted batch is dropped without commit; the Fjall database state is unchanged.
- The runtime caller (`vb_runtime`) observes the typed error at the journal-batch API boundary. The contract does not change runtime-level behavior; the runtime already surfaces typed errors to the operator.