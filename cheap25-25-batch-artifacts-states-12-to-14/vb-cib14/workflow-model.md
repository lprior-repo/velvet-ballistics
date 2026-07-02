# Workflow Model: vb-cib14 — Wire RuntimeJournalEvent::Resumed → JournalEvent::RunResumed

## Dispatch Workflow (StorageRuntimeJournal::storage_event)

```text
StorageRuntimeJournal::storage_event(event: RuntimeJournalEvent, seq: EventSeq)
    -> RuntimeResult<JournalEvent>

1. match &event:
     RunSubmitted | RunAdmission | RunFinished | RunFailed | RunCancelled | RunKilled
       | StepStarted | StepSucceeded
       -> run_storage_event(clone_for_dispatch(&event), seq)
     ActionScheduled | ActionCompleted | ActionScheduledTicket | ActionCompletedEnvelope
       | ActionFailed | ActionAbandoned
       -> action_storage_event(clone_for_dispatch(&event), seq)
     WaitScheduled | WaitResolved | AskScheduled | AskAnswered | AskTimedOut
       | SlotWritten | Resumed
       -> boundary_storage_event(clone_for_dispatch(&event), seq)?

2. boundary_storage_event dispatches on cloned event:
     WaitScheduled  -> Ok(Some(WaitScheduledEvent { run, seq, step, attempt: 1 }))
     WaitResolved   -> Ok(Some(WaitResolvedEvent  { run, seq, step, attempt: 1 }))
     AskScheduled   -> Ok(Some(AskScheduledEvent  { run, seq, step, attempt: 1 }))
     AskAnswered    -> Ok(Some(AskAnsweredEvent   { run, seq, step, attempt: 1 }))
     AskTimedOut    -> Ok(Some(AskTimedOutEvent   { run, seq, step, attempt: 1 }))
     SlotWritten    -> Ok(Some(SlotWrittenEvent   { run, seq, slot, value, extra, attempt: 1 }))
     Resumed { run, timestamp } (NEW)
                     -> Ok(Some(RunResumed {
                          run,
                          seq,
                          timestamp: convert_resume_timestamp(timestamp, run)?,
                        }))
     Other variants -> Ok(None)  (explicit no-op)

3. If Some(stored_event) -> return Ok(stored_event).
   (No fallback to RunFailedEvent after this fix; once vb-edvbj deletes the catch-all,
    exhaustive match is the only termination path.)
```

## Resume Success Workflow (End-to-End)

```text
Caller: Runtime API -> Shard::handle_resume(run) (already implemented, unchanged)

  1. validate_run_exists(run)        -> Ok or Err(RunIdNotFound)
  2. already-running short-circuit   -> Ok(ResumeResult { status: AlreadyRunning, timestamp })
  3. not-resumable guard              -> Err(NotResumable { run, current_state })
  4. apply(RuntimeEvent::Resume)     -> RuntimeState::Resuming (state machine, unchanged)
  5. append_resumed_event(run):
       a. is_run_tracked(run)        -> Err(IncompleteHydration) if not
       b. apply(RuntimeEvent::Resume) (already done by step 4)
       c. timestamp := current_timestamp()                  // u64 seconds since UNIX epoch
       d. RuntimeJournalEvent::Resumed { run, timestamp }   // emitted
       e. self.append_journal_event(resumed_event)          // journal append
          on failure: apply(RuntimeEvent::ResumeRollback)   // restore Resumable state
                    -> Err(JournalAppendFailedWithSource)
       f. Ok(timestamp)
  6. drive_run(run)                  -> result captured
  7. observe_resume_drive_result     -> typed outcome or roll back
  8. Ok(ResumeResult { status: Resumed, timestamp })

Journal path (NEW for this bead):
  self.append_journal_event(RuntimeJournalEvent::Resumed { run, timestamp })
    -> StorageRuntimeJournal::append_sequenced(event, seq)
       -> StorageRuntimeJournal::storage_event(event, seq)?
            -> boundary_storage_event(clone_for_dispatch(&event), seq)?
                 -> Ok(Some(JournalEvent::RunResumed {
                       run,
                       seq,
                       timestamp: convert_resume_timestamp(timestamp, run)?,
                     }))
       -> self.append_storage_event(&storage_event)?
            -> self.journal.append_journaled(event) or self.journal.append_strict(event)
```

## Resume Failure Workflow (Conversion Overflow)

```text
StorageRuntimeJournal::storage_event(
    RuntimeJournalEvent::Resumed { run, timestamp }, seq
) where timestamp > i64::MAX:

  1. dispatch routes to boundary_storage_event
  2. boundary_storage_event reaches the Resumed arm
  3. convert_resume_timestamp(timestamp, run):
       i64::try_from(timestamp) -> Err
       -> Err(RuntimeError::ResumeTimestampOverflow { run, timestamp })
  4. boundary_storage_event propagates the error via `?`
  5. storage_event propagates via `?`
  6. append_sequenced returns Err(ResumeTimestampOverflow)
  7. Shard::append_resumed_event catches the error and:
       - applies RuntimeEvent::ResumeRollback
       - returns Err(ResumeError::JournalAppendFailedWithSource(source))
  8. Shard::handle_resume returns Err(ResumeError::JournalAppendFailedWithSource(...))
  9. No journal entry is appended. No RunFailedEvent is synthesized.
 10. The run remains in RuntimeState::Resumable so retry is possible.

NOTE: For all realistic UNIX timestamps (current era < 2^31 << i64::MAX), this path
      is unreachable. It exists as a typed boundary for hostile-input/long-running
      system safety and as a Verus refinement target.
```

## Resume Failure Workflow (Storage Append Fails After Conversion Succeeds)

```text
StorageRuntimeJournal::storage_event returns Ok(RunResumed) but
append_storage_event fails (e.g. Fjall IO error):

  1. StorageRuntimeJournal::append_sequenced returns
     Err(RuntimeError::from(journal_append_error))
  2. Shard::append_resumed_event catches the journal append error and:
       - applies RuntimeEvent::ResumeRollback
       - returns Err(ResumeError::JournalAppendFailedWithSource(...))
  3. The in-memory RuntimeState::Resuming is restored to Resumable.
  4. No RunResumed journal entry is durable; resume can be retried.

This path is unchanged by the vb-cib14 fix; it is included here so the workflow
model captures the full failure matrix for the resumed event.
```

## Recovery / Replay Workflow (Downstream Consumers)

```text
Recovery/replay reads JournalEvent::RunResumed from the journal:

  - crates/vb_storage/src/journal/incident.rs:203
      lifecycle_state(RunResumed) -> LifecycleState::Active
  - crates/vb_storage/src/recovery/hydrate.rs:754
      is_in_flight_or_completed(RunResumed) -> Ok(false)
      (i.e. "not yet finished/failed/cancelled")
  - crates/vb_storage/src/recovery/replay/observation/normalize.rs:60,126
      RunResumed is classified for recovery observation
  - crates/vb_storage/src/recovery/replay/summary/apply.rs:79-81
      RunResumed is treated as a lifecycle event with no per-event sequence
      history concern at the summary level

  - CLI surfaces that already assume the variant exists:
      crates/vb_cli/src/lifecycle.rs:150-220  (writes RunResumed directly)
      crates/vb_cli/src/commands_journal.rs:264
      crates/vb_cli/src/status.rs:417
      crates/vb_cli/src/commands_diff.rs:205,236
      crates/vb_cli/src/events.rs:171
      crates/workspace_tests/tests/vb_test_cli_storage_io_behavior.rs:225

These consumers are already correct; they require the production dispatcher
to actually emit RunResumed so the recovery state transitions to Active.
```

## Temporal Invariants

- **Single-Event Invariant (per run per resume attempt):** exactly one `RuntimeJournalEvent::Resumed` is appended per successful resume attempt. The mapper must not duplicate it and must not synthesize a parallel event.
- **Boundary Order Invariant:** the storage append of `RunResumed` happens before `Shard::handle_resume` returns `Ok(Resumed)`. A storage append failure must propagate before the shard commits the resume as successful.
- **Conversion Determinism Invariant:** for any `u64` value, `convert_resume_timestamp` either succeeds with a deterministic chrono `DateTime<Utc>` or returns `Err(ResumeTimestampOverflow { run, timestamp })` with the original `u64` carried for diagnostics.
- **Totality Invariant (paired with vb-edvbj):** `storage_event` is exhaustive over `RuntimeJournalEvent`. The dispatch table must be total, and once the catch-all `RunFailedEvent` fallback is removed by vb-edvbj, this invariant is the only thing standing between us and a compile error.
- **Recovery-State Invariant:** emitting `RunResumed` causes `incident.rs::lifecycle_state` to return `LifecycleState::Active`. Emitting `RunFailedEvent` (the bug) causes it to return `LifecycleState::Failed`. The fix removes the path that produces `Failed` for an actually-resumed run.
- **Single-Clone Invariant (preserved):** for any `RuntimeJournalEvent`, `storage_event` clones the event exactly once. The `Resumed` arm must not introduce a second clone.

## Workflow Hazards (summary)

- Boundary dispatch race: `Resumed` is currently in `boundary_storage_event`'s no-op catch-all; if vb-edvbj deletes the outer `RunFailedEvent` fallback before this fix lands, the dispatch becomes a compile error.
- `DateTime::<Utc>::from_timestamp` returns `Option<DateTime<Utc>>`. For legal `i64` seconds values it is `Some(_)`; for far-future values it is `None`. The mapper must handle the `None` case explicitly.
- The `seq` parameter is per-run monotonic. The mapper must never invent or decrement it.
- `EventSeq::MAX` (u64::MAX) is the overflow sentinel; the mapper must not produce a `RunResumed` with `seq == EventSeq::MAX` even if asked (caller's responsibility, but worth documenting).
- The shard already calls `apply(RuntimeEvent::ResumeRollback)` on journal append failure, so the ResumeStateMachine refinement obligation (RRO-TLA-RESUME-001) still holds after the mapper arm is added.