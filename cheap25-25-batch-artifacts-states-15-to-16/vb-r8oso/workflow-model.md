# Workflow Model — vb-r8oso

**bead_id:** vb-r8oso
**owner_stage:** rust-contract
**upstream_artifacts:** `domain-model.md`, `type-contracts.md`

This artifact fixes the legal states, transitions, guards, outcomes, terminal
states, retries, cancellation, and idempotence for every append path affected
by the new `next_sequence_at_write` guard. It is the surface on which the
hazard analysis and the proof seeds are built.

---

## 1. Legal States Per Run

For a single `RunId`, the journal exists in exactly one of the following
states at any instant in this process:

| State | Definition | Applies |
|---|---|---|
| `Fresh` | No event for `run` is present in the `events` keyspace. | After `open`, before any append. |
| `Tail(n)` | The events for `run` are a strict prefix `[0..n]` with no holes. | After `n` successful appends. |
| `TailHole(n)` (transient, observable only) | The events for `run` contain a hole — strictly FS-1 territory. **Reachability:** prior to the fix, an in-process append could durably land a `seq > len(events_for_run(run))` and create this state; under the fix, this state is **not reachable** by any in-process append path. It can only be observed if on-disk corruption pre-existed (e.g., a previous-version crash). | Replay-time only, surfaced by `events_for_run`. |

The fix's invariant: every successful append is a transition
`Fresh → Tail(1)` or `Tail(n) → Tail(n+1)`. The transition `Fresh → TailHole(...)`
is forbidden. The transition `Tail(n) → Tail(n+k)` for `k > 1` is forbidden
(no silent gap).

## 2. The Five Affected Append Workflows

### 2.1 `append_journaled(event)` (single event, no SyncAll)

```
   Pre:  event is well-formed; event.run_id() != RunId::ZERO
         event.is_valid() holds
         caller's event.seq() is some value S
   Try:  expected := next_sequence_at_write(event.run_id())
         S == expected?
           Y -> proceed to durable duplicate + index mutation + commit
           N -> Err(SequenceMismatch { run, expected, S })
   Post (Ok): event durably visible at (run, S);
              persistent state tail advances to Tail(n+1)
   Post (Err): no durable change; persistent state stays at Tail(n)
   Idem: a retry of an Ok transition that crashed pre-return yields
         DuplicateEvent (today) or SequenceMismatch (post-fix under
         a partial commit guard). The batched `append_strict` is
         preferred over `append_journaled` when durability matters.
```

### 2.2 `append_strict(event)` (single event, force SyncAll)

```
   Pre:  same as 2.1
   Try:  expected := next_sequence_at_write(event.run_id())
         S == expected?
           Y -> durable duplicate pre-check, batch stage, batch.strict().commit()
           N -> Err(SequenceMismatch { run, expected, S })
   Post (Ok): event durably visible AND force-fsync'd atomically;
              strict durability boundary observed
   Post (Err): no durable change; no fsync performed; state unchanged
   Idem: a retry after a StrictDurabilityFailed failure must re-observe
         the seq invariant; failure path must NOT downgrade to
         DuplicateEvent silently.
```

### 2.3 `append_strict_batch(events)` (slice of events, one SyncAll)

```
   Pre:  events.len() > 0
         each event is well-formed; event.run_id() != RunId::ZERO
   Body: maintain a per-run running last_seq during iteration
         for each event E in events:
           expected := next_sequence_at_write(E.run_id())
           if E.seq() != expected:
               Err(SequenceMismatch { run, expected, actual: E.seq() })
               -> BATCH ABORTED HERE; nothing durably committed
           else:
               batch.append_event(E)   # which itself can also fail
         batch.strict().commit()        # one SyncAll
   Post (Ok): every event durable; state advances Tail(n) -> Tail(n+k)
   Post (Err): batch fully aborted (no partial commit)
   Idem: a retry of an Err must re-evaluate expectations against the
         current durable state. The C6 precedence guarantees that an
         abort does not leave a half-committed batch.
```

### 2.4 `append_unfsynced(event)` (`pub(crate)`, lowest-level)

```
   Pre:  write_lock acquired
   Try:  expected := next_sequence_at_write(event.run_id())
         S == expected?
           Y -> duplicate check, encode, batch (event + pending-action-index)
               commit
           N -> Err(SequenceMismatch { run, expected, S })
   Post: same as 2.1 but at the lowest level
   Idem: the write_lock serializes callers; spurious retries due to
         crashes are caught by DuplicateEvent (unchanged) for an event
         that did commit, or by SequenceMismatch / InvalidEvent for
         one that did not.
```

### 2.5 `JournalWriteBatch::append_event(event)` (in-batch path)

```
   Pre:  batch is open (not aborted); well-formed event
   Body: # C6 guard precedence updated:
         1. construct key
         2. event.is_valid()
         3. NEW: expected := journal.next_sequence_at_write(event.run_id())
                 if event.seq() != expected:
                     self.aborted = true
                     return Err(SequenceMismatch { run, expected, actual: event.seq() })
         4. self.staged_event_keys.contains(key) -> DuplicateStagedKey
         5. journal.events.contains_key(key) -> DuplicateEvent + abort
         6. count capacity -> QueueFull
         7. encode / payload size -> PayloadTooLarge
         8. byte admission -> JournalBatchBytesExceeded
         9. inner.insert(...)
   Post (Ok): event staged; staged_event_keys updates; commit later
   Post (Err at step 3..6): batch aborted / state depends on guard
   Idem: a batch that aborts at step 3 commits nothing
```

## 3. Recovery / Replay Workflow (NOT MODIFIED)

`events_for_run(R)` is unchanged. After the fix, it CANNOT observe `TailHole`
state created during this process's lifetime. A `TailHole` observed at
recovery time indicates the on-disk state predates the fix or was
manipulated outside the typed API; the recovery path continues to surface
it as `JournalError::SequenceGap`.

This separation is intentional: `SequenceGap` remains the legitimate
diagnostic for *read-time* detection of an on-disk sequence hole; the
in-process workflow never creates the hole so it never produces
`SequenceGap` under the fix.

## 4. Cancellation, Shutdown, Drain

The new guard has no cancellation surface. Existing journal shutdown
handles unblock writers via `QueueShutdown`; this path is unchanged. The
new method `next_sequence_at_write` does not block; if the Fjall LSM
lookup fails, the failure surfaces as `JournalError::Fjall(_)` to the
caller, which then handles the recovery choice.

Drain semantics: if a writer queue drains mid-batch, the next call to
`flush_profile` resumes from the durable tail. The next event in the
batch is validated against the post-drain tail; a stale seq is rejected
with `SequenceMismatch`.

## 5. Idempotence Matrix

| Caller retry scenario | Surface today | Surface post-fix |
|---|---|---|
| Caller retries a successful `append_journaled` because of network/process loss AFTER the event was already committed | `DuplicateEvent` (unchanged) | `DuplicateEvent` (unchanged; the new guard never fires for a duplicate because the durable tail is `n+1` and the caller's seq is `n+1` or earlier) |
| Caller retries because `persist_strict` failed after the batch was committed | `DuplicateEvent` (unchanged) | `DuplicateEvent` (unchanged) |
| Caller writes a stale `seq` because of an in-memory counter drift after recovery | Event durably committed; later observed as `SequenceGap` at replay | `Err(SequenceMismatch { run, expected, actual })` at write time, no durable change |
| Caller writes a too-large `seq` (skip-ahead) | Event durably committed; hole later observed as `SequenceGap` | `Err(SequenceMismatch { run, expected, actual })` at write time, no durable change |
| Two callers race on the same `(run, seq)` (out of scope; only one writer per process) | One writes; the other observes `DuplicateEvent` or wins the race due to Fjall `insert` overwriting | Same; the new guard does not affect this because the second caller's seq is `n+1` too, not mismatched against `next_sequence_at_write`. |

## 6. Retry / Backoff Policies (Out of Scope, Caller-Owned)

- The fix does not introduce a backoff policy. Callers that receive `SequenceMismatch` should treat the error as deterministic and not retry; the caller's seq source is wrong and retrying with the same seq fails again.
- A caller that has reasons to retry with a re-derived seq (e.g., it learned about a missing event from a separate recovery scan) MUST re-call `next_sequence_at_write` immediately before the retry.

## 7. Failure Modes and Surfaces

```
                          Append path is called with seq != next_sequence_at_write(run)
                                          |
                                          v
                              Err(JournalError::SequenceMismatch {
                                  run, expected: next_sequence_at_write(run),
                                  actual: event.seq()
                              })
                                          |
                                          v
                       Diagnostic code = 0x4042
                       Symbolic code   = JOURNAL_SEQUENCE_MISMATCH_AT_WRITE
                                          |
                                          v
                                    No durable change
                                    Caller bug observed & reported
```

```
                          Caller computes a fresh seq from runtime allocator
                              e.g., EventSeq::new(allocator_counter.get())
                                          |
                                          v
                              seq == next_sequence_at_write(run)
                                          |
                                          v
                                  Continue append path -> Ok(()) / Err(other)
```

```
                          Append path succeeds, then process crashes pre-return
                                          |
                                          v
                          On recovery: events.contains_key(key) -> DuplicateEvent
                              (caller retry observes DuplicateEvent, NOT SequenceMismatch
                              because seq matches the durable tail, which is already n+1)
```

## 8. Terminal States

- **`Ok(())`** on an append is terminal for that single write. The durable state advances monotonically; there is no "undo" in this layer. Recovery-driven unwinding happens at higher layers and is out of scope.
- **`Err(SequenceMismatch)`** is terminal for that single write attempt. The durable state is unchanged. The caller must debug.
- **`Err(SequenceOverflow)`** is terminal for the run: the durable state has reached `EventSeq::MAX` and no further appends are possible. The caller must migrate off this run or fail the run.

## 9. Cross-Workflow / Cross-Run Considerations

- `next_sequence_at_write` is `per-run`. A caller that batches events across multiple runs (rare but possible in `append_strict_batch` if the slice spans runs) MUST call `next_sequence_at_write` for each run independently. The implementation cannot rely on a single observed `seq` for the batch.
- Multi-process writers are not supported (process-level lock); the cross-run race surfaces as `ProcessLockHeld`, unchanged.

## 10. Acceptance: Workflow-Level Tests

Per `codebase-map.md` §7, the following behavior tests must close the fix:

1. `append_strict_rejects_sequence_skipped_with_typed_error` — append seq=0, attempt seq=2, assert `Err(SequenceMismatch { expected: 1, actual: 2 })` directly from `append_strict`.
2. `append_strict_rejects_sequence_at_zero_for_run_with_history` — append 0..5, attempt seq=0, assert `SequenceMismatch { expected: 6, actual: 0 }`.
3. `append_strict_accepts_first_seq_for_fresh_run` — fresh journal, seq=0 succeeds.
4. `next_sequence_at_write_returns_zero_for_fresh_run` — public API contract.
5. `next_sequence_at_write_returns_last_plus_one_after_writes` — public API contract.
6. `append_strict_batch_rejects_on_first_mismatch_atomically` — batch with `[seq=3, seq=4]` after seq=2 stored; whole batch rejected with `SequenceMismatch`; `events_for_run` shows no new events written.
7. `append_unfsynced_uses_next_sequence_at_write_guard` — same guard reachable via the lower-level path (covers `runtime::append_sequenced`).
8. Kani harness gated behind `kani-sequence-at-write` Cargo feature.

These tests are emitted as **seeds** by this stage; `test-planner` owns the final test-file layout.
