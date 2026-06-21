# RE-019: Unhandled runtime journal events are persisted as `RunFailedEvent`

- **Severity**: High
- **Category**: bug
- **Location**: `crates/vb_runtime/src/journal/chunk_002.rs:248-280`
- **Confidence**: confirmed

## Description

`StorageRuntimeJournal::storage_event` turns any runtime event not mapped by the run, action, or boundary converters into `JournalEvent::RunFailedEvent`. Runtime events such as `WaitCancelled`, `AskCancelled`, and `Resumed` are therefore recorded as terminal run failures.

## Evidence

`boundary_storage_event` explicitly returns `Ok(None)` for non-failure boundary events at `crates/vb_runtime/src/journal/chunk_002.rs:248-263`:

```rust
RuntimeJournalEvent::RunSubmitted { .. }
| RuntimeJournalEvent::RunAdmission { .. }
| RuntimeJournalEvent::RunFinished { .. }
| RuntimeJournalEvent::RunFailed { .. }
| RuntimeJournalEvent::RunCancelled { .. }
| RuntimeJournalEvent::RunKilled { .. }
| RuntimeJournalEvent::ActionScheduled { .. }
| RuntimeJournalEvent::ActionCompleted { .. }
| RuntimeJournalEvent::ActionScheduledTicket { .. }
| RuntimeJournalEvent::ActionCompletedEnvelope { .. }
| RuntimeJournalEvent::ActionFailed { .. }
| RuntimeJournalEvent::WaitCancelled { .. }
| RuntimeJournalEvent::AskCancelled { .. }
| RuntimeJournalEvent::StepStarted { .. }
| RuntimeJournalEvent::StepSucceeded { .. }
| RuntimeJournalEvent::Resumed { .. } => Ok(None),
```

`storage_event` then converts `None` into a failed terminal event at `crates/vb_runtime/src/journal/chunk_002.rs:274-280`:

```rust
match Self::boundary_storage_event(event.clone(), seq)? {
    Some(storage_event) => Ok(storage_event),
    None => Ok(JournalEvent::RunFailedEvent {
        run: event.run_id(),
        seq,
        attempt: 1,
    }),
}
```

`RuntimeJournalEvent::Resumed` is documented as "Run was resumed from a suspended state" in `crates/vb_runtime/src/journal/chunk_001.rs:185-191`, not as a failure. `WaitCancelled` and `AskCancelled` are boundary cancellations, not `RunFailed`.

## Adversarial Check

Some of the `Ok(None)` variants are already handled by earlier converter functions, but `WaitCancelled`, `AskCancelled`, and `Resumed` are not. The fallback cannot distinguish "handled earlier" from "unrepresentable event," so it manufactures a failure for live non-failure events. A durable journal must never encode an unknown event as a terminal failure just to keep the type total.

## Suggested Fix

Add storage `JournalEvent` variants for wait cancellation, ask cancellation, and resume; or return an explicit unsupported-event error for events that cannot be represented. Do not use `RunFailedEvent` as a catch-all fallback.
