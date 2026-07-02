#![forbid(unsafe_code)]
//! Contract tests for vb-3wn7x: pending action index maintenance in the
//! runtime journal path.
//!
//! Master §18 requires `index_action` to be the authoritative pending
//! action cursor. The defect captured under vb-3wn7x was that the
//! runtime's `append_journaled` / `append_strict` path only wrote the
//! `run_event` record, leaving the index keyspace out of sync with the
//! durable event log.
//!
//! These tests exercise every action-lifecycle event variant through
//! every admission path that previously missed the index update:
//!
//! - `FjallJournal::append_journaled` (journaled profile)
//! - `FjallJournal::append_strict` (strict profile)
//! - `FjallJournal::append_strict_batch` (atomic batch insert)
//! - `JournalWriteBatch::append_event` (cross-keyspace batch)
//! - `JournalWriterQueue::drain_all` (queued writer)
//!
//! Insert-side events:
//! - `JournalEvent::ActionScheduled`
//! - `JournalEvent::ActionScheduledTicket`
//!
//! Remove-side events:
//! - `JournalEvent::ActionCompletedEvent`
//! - `JournalEvent::ActionCompletedEnvelope`
//! - `JournalEvent::ActionFailedEvent`
//! - `JournalEvent::ActionAbandoned`
//!
//! No-op events (run/step/wait/ask/slot/lifecycle): leave the index
//! untouched.

#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod tests {
    use crate::{
        DurableActionOutcome, EventSeq, FjallJournal, IndexStatusState, JournalError, JournalEvent,
        JournalWriteBatch, JournalWriterQueue, StorageLimits,
    };
    use std::sync::Arc;
    use vb_core::{
        ActionId, ActionTicket, RunId, SeqNo, SlotIdx, StepIdx as CoreStepIdx, Taint,
        WorkflowDigest, WorkflowId, ids::StepIdx,
    };

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation must succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open must succeed");
        (temp, journal)
    }

    fn temp_journal_with_queue() -> (tempfile::TempDir, FjallJournal, Arc<JournalWriterQueue>) {
        let (temp, journal) = temp_journal();
        let queue = Arc::new(
            JournalWriterQueue::new(8, 4, StorageLimits::DEFAULT)
                .expect("queue construction must succeed"),
        );
        (temp, journal, queue)
    }

    fn count_action_index_entries(journal: &FjallJournal) -> usize {
        journal.index_action.iter().count()
    }

    fn has_index_entry(
        journal: &FjallJournal,
        action: ActionId,
        run: RunId,
        step: CoreStepIdx,
    ) -> bool {
        let key = crate::keys::index_action_key(action, run, step)
            .expect("index_action_key must succeed for valid inputs");
        journal
            .has_action_index_entry(key)
            .expect("has_action_index_entry must succeed")
    }

    fn minimal_action_ticket(run: RunId, step: StepIdx, action: ActionId) -> ActionTicket {
        ActionTicket {
            run,
            step,
            seq: SeqNo::ZERO,
            action,
            attempt: 1,
            idempotency_key: 0,
            capacity: 1,
        }
    }

    fn fabricated_workflow_id_from_digest_for_negative_check(digest: WorkflowDigest) -> WorkflowId {
        let [b0, b1, b2, b3, ..] = digest.as_bytes();
        WorkflowId::new(u32::from_be_bytes([b0, b1, b2, b3]))
    }

    // -----------------------------------------------------------------
    // Direct journal path: append_journaled (write_lock-bound)
    // -----------------------------------------------------------------

    #[test]
    fn append_journaled_action_scheduled_creates_index_entry() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1);
        let step = StepIdx::new(2);
        let action = ActionId::new(3);

        let event = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        };
        journal
            .append_journaled(&event)
            .expect("append_journaled must succeed");

        assert!(
            has_index_entry(&journal, action, run, step),
            "ActionScheduled must add a pending action marker at (action, run, step)"
        );
        assert_eq!(
            count_action_index_entries(&journal),
            1,
            "index_action must contain exactly one entry",
        );
    }

    #[test]
    fn append_journaled_action_completed_removes_index_entry() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2);
        let step = StepIdx::new(5);
        let action = ActionId::new(11);

        journal
            .append_journaled(&JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(0),
                step,
                action,
                attempt: 1,
            })
            .expect("schedule must succeed");
        assert!(
            has_index_entry(&journal, action, run, step),
            "marker must exist before completion"
        );

        journal
            .append_journaled(&JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(1),
                step,
                action,
                attempt: 1,
            })
            .expect("completion must succeed");
        assert!(
            !has_index_entry(&journal, action, run, step),
            "ActionCompletedEvent must remove the pending action marker"
        );
        assert_eq!(
            count_action_index_entries(&journal),
            0,
            "index_action must be empty after terminal completion",
        );
    }

    #[test]
    fn append_journaled_action_failed_removes_index_entry() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3);
        let step = StepIdx::new(7);
        let action = ActionId::new(13);

        journal
            .append_journaled(&JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(0),
                step,
                action,
                attempt: 1,
            })
            .expect("schedule must succeed");
        journal
            .append_journaled(&JournalEvent::ActionFailedEvent {
                run,
                seq: EventSeq::new(1),
                step,
                action,
                attempt: 1,
            })
            .expect("failure must succeed");
        assert!(
            !has_index_entry(&journal, action, run, step),
            "ActionFailedEvent must remove the pending action marker"
        );
    }

    // -----------------------------------------------------------------
    // Direct journal path: append_strict (force fsync)
    // -----------------------------------------------------------------

    #[test]
    fn append_strict_action_scheduled_creates_index_entry() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(4);
        let step = StepIdx::new(8);
        let action = ActionId::new(17);

        journal
            .append_strict(&JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(0),
                step,
                action,
                attempt: 1,
            })
            .expect("append_strict must succeed");
        assert!(
            has_index_entry(&journal, action, run, step),
            "append_strict ActionScheduled must add the index marker"
        );
    }

    // -----------------------------------------------------------------
    // Atomic batch insert path: append_event through JournalWriteBatch
    // -----------------------------------------------------------------

    #[test]
    fn batch_append_event_action_scheduled_creates_index_entry() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(5);
        let step = StepIdx::new(9);
        let action = ActionId::new(19);

        let event = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        };
        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("commit must succeed");

        assert!(
            has_index_entry(&journal, action, run, step),
            "batch-staged ActionScheduled must add the pending action marker"
        );
    }

    #[test]
    fn batch_append_event_action_completed_removes_index_entry() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(6);
        let step = StepIdx::new(10);
        let action = ActionId::new(23);

        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .append_event(&JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(0),
                step,
                action,
                attempt: 1,
            })
            .expect("schedule must succeed");
        batch
            .append_event(&JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(1),
                step,
                action,
                attempt: 1,
            })
            .expect("completion must succeed");
        batch.commit().expect("commit must succeed");

        assert!(
            !has_index_entry(&journal, action, run, step),
            "batch-staged ActionCompleted must remove the pending action marker"
        );
        assert_eq!(
            count_action_index_entries(&journal),
            0,
            "no markers must remain after scheduled+completed pair",
        );
    }

    #[test]
    fn batch_append_event_action_scheduled_ticket_creates_index_entry() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(7);
        let step = StepIdx::new(11);
        let action = ActionId::new(29);
        let ticket = minimal_action_ticket(run, step, action);

        let event = JournalEvent::ActionScheduledTicket {
            run,
            seq: EventSeq::new(0),
            ticket,
            input: SlotIdx::new(0),
            output: SlotIdx::new(1),
            action_abi_digest: WorkflowDigest::from_bytes([0xAB; 32]),
        };
        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .append_event(&event)
            .expect("batch.append_event must succeed");
        batch.commit().expect("commit must succeed");

        assert!(
            has_index_entry(&journal, action, run, step),
            "ActionScheduledTicket must add the pending action marker"
        );
    }

    #[test]
    fn batch_append_event_action_completed_envelope_removes_index_entry() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(8);
        let step = StepIdx::new(12);
        let action = ActionId::new(31);
        let ticket = minimal_action_ticket(run, step, action);

        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .append_event(&JournalEvent::ActionScheduledTicket {
                run,
                seq: EventSeq::new(0),
                ticket,
                input: SlotIdx::new(0),
                output: SlotIdx::new(1),
                action_abi_digest: WorkflowDigest::from_bytes([0xAB; 32]),
            })
            .expect("schedule must succeed");
        batch
            .append_event(&JournalEvent::ActionCompletedEnvelope {
                run,
                seq: EventSeq::new(1),
                ticket,
                output: SlotIdx::new(1),
                outcome: DurableActionOutcome::Ready,
                value: Vec::new(),
                encoded_len: 0,
                taint: Taint::Clean,
                value_digest: [0u8; 32],
                action_abi_digest: WorkflowDigest::from_bytes([0xAB; 32]),
            })
            .expect("completion must succeed");
        batch.commit().expect("commit must succeed");

        assert!(
            !has_index_entry(&journal, action, run, step),
            "ActionCompletedEnvelope must remove the pending action marker"
        );
    }

    #[test]
    fn batch_append_event_action_abandoned_removes_index_entry() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(9);
        let step = StepIdx::new(13);
        let action = ActionId::new(37);
        let ticket = minimal_action_ticket(run, step, action);

        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .append_event(&JournalEvent::ActionScheduledTicket {
                run,
                seq: EventSeq::new(0),
                ticket,
                input: SlotIdx::new(0),
                output: SlotIdx::new(1),
                action_abi_digest: WorkflowDigest::from_bytes([0xAB; 32]),
            })
            .expect("schedule must succeed");
        batch
            .append_event(&JournalEvent::ActionAbandoned {
                run,
                seq: EventSeq::new(1),
                ticket,
            })
            .expect("abandonment must succeed");
        batch.commit().expect("commit must succeed");

        assert!(
            !has_index_entry(&journal, action, run, step),
            "ActionAbandoned must remove the pending action marker"
        );
    }

    #[test]
    fn batch_append_event_non_action_events_do_not_touch_index() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(10);
        let workflow = WorkflowDigest::from_bytes([0xCD; 32]);

        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .append_event(&JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow,
            })
            .expect("RunAccepted must succeed");
        batch
            .append_event(&JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(1),
                step: StepIdx::new(0),
                attempt: 1,
            })
            .expect("StepStarted must succeed");
        batch
            .append_event(&JournalEvent::StepSucceeded {
                run,
                seq: EventSeq::new(2),
                step: StepIdx::new(0),
                output: SlotIdx::new(2),
            })
            .expect("StepSucceeded must succeed");
        batch.commit().expect("commit must succeed");

        assert_eq!(
            count_action_index_entries(&journal),
            0,
            "non-action events must not introduce spurious index entries",
        );
    }

    #[test]
    fn batch_append_event_run_accepted_does_not_fabricate_admission_indexes() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(10_001);
        let seq = EventSeq::new(55);
        let workflow = WorkflowDigest::from_bytes([0xEF; 32]);

        let mut batch = JournalWriteBatch::new(&journal);
        batch
            .append_event(&JournalEvent::RunAccepted { run, seq, workflow })
            .expect("RunAccepted append_event must succeed");
        batch.commit().expect("commit must succeed");

        assert_eq!(
            journal
                .run_header(run)
                .expect("run_header lookup must succeed"),
            None,
            "RunAccepted event alone must not synthesize a run header",
        );
        let fabricated_workflow_id =
            fabricated_workflow_id_from_digest_for_negative_check(workflow);
        let status_key = crate::keys::index_status_key(IndexStatusState::Submitted, seq.get(), run)
            .expect("status key construction must succeed");
        let workflow_key = crate::keys::index_workflow_key(fabricated_workflow_id, run)
            .expect("workflow key construction must succeed");
        assert!(
            !journal
                .has_status_index_entry(status_key)
                .expect("status index lookup must succeed"),
            "RunAccepted event alone must not use seq as an admission timestamp",
        );
        assert!(
            !journal
                .has_workflow_index_entry(workflow_key)
                .expect("workflow index lookup must succeed"),
            "RunAccepted event alone must not derive WorkflowId from digest bytes",
        );
    }

    // -----------------------------------------------------------------
    // Strict batch path: append_strict_batch
    // -----------------------------------------------------------------

    #[test]
    fn append_strict_batch_action_lifecycle_maintains_index() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(11);
        let step = StepIdx::new(14);
        let action = ActionId::new(41);

        let scheduled = JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        };
        let completed = JournalEvent::ActionCompletedEvent {
            run,
            seq: EventSeq::new(1),
            step,
            action,
            attempt: 1,
        };

        journal
            .append_strict_batch(&[scheduled])
            .expect("first batch must succeed");
        assert!(
            has_index_entry(&journal, action, run, step),
            "ActionScheduled in strict batch must add the marker",
        );

        journal
            .append_strict_batch(&[completed])
            .expect("second batch must succeed");
        assert!(
            !has_index_entry(&journal, action, run, step),
            "ActionCompletedEvent in strict batch must remove the marker",
        );
    }

    // -----------------------------------------------------------------
    // Queued writer path: drain_all through JournalWriterQueue
    // -----------------------------------------------------------------

    #[test]
    fn queued_flush_batch_action_lifecycle_maintains_index() {
        let (_temp, journal, queue) = temp_journal_with_queue();
        let run = RunId::new(12);
        let step = StepIdx::new(15);
        let action = ActionId::new(43);

        queue
            .enqueue_journaled(JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(0),
                step,
                action,
                attempt: 1,
            })
            .expect("enqueue schedule must succeed");
        queue
            .enqueue_journaled(JournalEvent::ActionCompletedEvent {
                run,
                seq: EventSeq::new(1),
                step,
                action,
                attempt: 1,
            })
            .expect("enqueue complete must succeed");

        // Drain the queue into the journal — the event AND its index
        // mutation must be visible after the drain.
        let report = queue.drain_all(&journal).expect("drain_all must succeed");
        assert_eq!(report.written, 2, "queue must report 2 events flushed");

        assert!(
            !has_index_entry(&journal, action, run, step),
            "queued + drained lifecycle pair must leave the index empty",
        );
        assert_eq!(
            count_action_index_entries(&journal),
            0,
            "queued lifecycle pair must not leave stale pending entries",
        );
    }

    #[test]
    fn queued_flush_batch_action_scheduled_alone_creates_index_entry() {
        let (_temp, journal, queue) = temp_journal_with_queue();
        let run = RunId::new(13);
        let step = StepIdx::new(16);
        let action = ActionId::new(47);

        queue
            .enqueue_journaled(JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(0),
                step,
                action,
                attempt: 1,
            })
            .expect("enqueue schedule must succeed");
        queue.drain_all(&journal).expect("drain_all must succeed");

        assert!(
            has_index_entry(&journal, action, run, step),
            "queued ActionScheduled must add the index marker after drain",
        );
    }

    // -----------------------------------------------------------------
    // Idempotency: duplicate events at same seq are rejected.
    // -----------------------------------------------------------------

    #[test]
    fn duplicate_action_scheduled_does_not_duplicate_index_entry() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(14);
        let step = StepIdx::new(17);
        let action = ActionId::new(53);

        // Successful first schedule: marker present.
        journal
            .append_journaled(&JournalEvent::ActionScheduled {
                run,
                seq: EventSeq::new(0),
                step,
                action,
                attempt: 1,
            })
            .expect("first schedule must succeed");
        assert!(has_index_entry(&journal, action, run, step));

        // Duplicate at the journal-event layer is rejected, so the
        // index is not asked to update twice. Direct path surfaces
        // JournalError::DuplicateEvent.
        let duplicate_result = journal.append_journaled(&JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        });
        assert!(
            matches!(duplicate_result, Err(JournalError::DuplicateEvent { .. })),
            "duplicate ActionScheduled at same seq must be rejected, got {:?}",
            duplicate_result,
        );
        assert!(
            has_index_entry(&journal, action, run, step),
            "rejected duplicate must not disturb existing marker",
        );
        assert_eq!(
            count_action_index_entries(&journal),
            1,
            "index must contain exactly one entry (Fjall last-write-wins)",
        );
    }
}
