use crate::shard::transitions::SnapshotWriteOutcome;
#[cfg(not(kani))]
use vb_storage::recovery::RunSnapshot;

impl Shard {
    /// Appends a journal event, buffering it during the coalesce window.
    ///
    /// When `coalesce_window_ticks` is 1, the event is written immediately
    /// and the per-run sequence is advanced atomically with the write.
    /// When greater than 1, the event is collected into `coalesce_buffer`
    /// with its assigned starting sequence. The per-run sequence is NOT
    /// advanced until [`flush_coalesce_buffer`] confirms the batch
    /// persisted successfully, so a partial flush failure leaves the
    /// in-memory sequence map consistent with the durable record (RQ-W0-19).
    #[cfg(not(kani))]
    pub(crate) fn append_journal_event(&mut self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        let run = event.run_id();
        let seq = self.journal_sequence_for(run);
        let _next_seq_check = seq
            .get()
            .checked_add(1)
            .map(EventSeq::new)
            .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;

        if self.current_coalesce_window_remaining > 0 {
            // Coalesce window is active: buffer the event with its starting
            // sequence. Sequence advancement is deferred to flush_coalesce_buffer
            // so a partial flush failure does not desynchronise the in-memory
            // sequence map from durable persistence.
            self.coalesce_buffer.push((event, seq));
        } else {
            // No coalescing: write immediately and advance the sequence.
            self.journal.append_sequenced(event, seq)?;
            let next_seq = EventSeq::new(
                seq.get()
                    .checked_add(1)
                    .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?,
            );
            self.journal_sequences.insert(run, next_seq);
        }
        Ok(())
    }

    /// `#[cfg(kani)]` replacement for `append_journal_event` that returns
    /// `Ok(())`. Trust boundary TB-vb282my-journal-stub-001.
    /// Production journal append uses Fjall-backed persistence; stubbed version
    /// avoids real I/O during Kani bounded model checking.
    /// NOTE(vb-k8ut.2): Changed from `kani::any()` to `Ok(())` to avoid
    /// `RuntimeError: kani::Arbitrary` requirement. The nondeterministic
    /// error-path exercise is not needed for IPC command reconciliation proofs
    /// and the `RuntimeError` type's complex fields (Box<CoreError>,
    /// Arc<JournalError>, Capability, etc.) make full Arbitrary impl impractical.
    #[cfg(kani)]
    pub(crate) fn append_journal_event(
        &mut self,
        _event: RuntimeJournalEvent,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    #[cfg(not(kani))]
    fn journal_sequence_for(&self, run: RunId) -> EventSeq {
        self.journal_sequences
            .get(&run)
            .copied()
            .unwrap_or(EventSeq::ZERO)
    }

    /// Flushes the coalesce buffer by writing all buffered events atomically
    /// via `RuntimeJournal::append_sequenced_batch`.
    ///
    /// This is called when the coalesce window expires (counter reaches zero).
    /// Each event is written with its recorded per-run starting sequence.
    ///
    /// On success the per-run `journal_sequences` are advanced past the last
    /// event assigned to each run, and the buffer is cleared. On failure,
    /// the per-run sequence map and the buffer remain unchanged so the next
    /// flush attempt reuses the same starting sequence (RQ-W0-19).
    #[cfg(not(kani))]
    pub(crate) fn flush_coalesce_buffer(&mut self) -> RuntimeResult<()> {
        if self.coalesce_buffer.is_empty() {
            return Ok(());
        }

        let events: Vec<RuntimeJournalEvent> = self
            .coalesce_buffer
            .iter()
            .map(|(event, _seq)| event.clone())
            .collect();

        // Use the first event's sequence as the batch start. The batch method
        // assigns contiguous sequences from seq_start, so we use the earliest
        // sequence in the buffer as the anchor point.
        let first_seq = self
            .coalesce_buffer
            .first()
            .map(|(_, seq)| *seq)
            .unwrap_or(EventSeq::ZERO);

        self.journal.append_sequenced_batch(&events, first_seq)?;

        // RQ-W0-19: advance per-run sequences only after the batch persists.
        // `append_sequenced_batch` assigns contiguous sequences from
        // `first_seq` per run, so the count of buffered events per run
        // determines the post-flush sequence.
        let mut count_per_run: std::collections::HashMap<RunId, usize> =
            std::collections::HashMap::new();
        for (event, _) in &self.coalesce_buffer {
            *count_per_run.entry(event.run_id()).or_insert(0) += 1;
        }
        for (run, count) in count_per_run {
            let current = self.journal_sequence_for(run);
            let advance = u64::try_from(count)
                .map_err(|_| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;
            let next = EventSeq::new(current.get().checked_add(advance).ok_or_else(|| {
                RuntimeError::from(vb_storage::JournalError::SequenceOverflow)
            })?);
            self.journal_sequences.insert(run, next);
        }

        self.coalesce_buffer.clear();
        Ok(())
    }

    /// `#[cfg(kani)]` replacement for `flush_coalesce_buffer` that returns
    /// `Ok(())`. Avoids real I/O during Kani bounded model checking.
    #[cfg(kani)]
    pub(crate) fn flush_coalesce_buffer(&mut self) -> RuntimeResult<()> {
        self.coalesce_buffer.clear();
        Ok(())
    }

    pub(crate) fn discard_journal_sequence(&mut self, run: RunId) {
        self.journal_sequences.swap_remove(&run);
    }

    /// Writes a run snapshot at the given executed-step threshold.
    ///
    /// This method:
    /// 1. Checks if snapshotting is enabled (`interval > 0`).
    /// 2. Checks if the step count has reached the trigger threshold.
    /// 3. Advances the journal sequence, writes the snapshot, and updates
    ///    the threshold. On failure, the sequence is rolled back.
    ///
    /// Returns the outcome of the write attempt.
    ///
    /// **Non-blocking contract (C-2 / C-3):** Serialization failures and
    /// journal write errors are converted to `SnapshotWriteOutcome::Failed`
    /// so the caller can continue the run lifecycle. Only the `Written`
    /// outcome carries a successful sequence advance.
    #[cfg(not(kani))]
    pub(crate) fn write_snapshot_for_run(
        &mut self,
        run: RunId,
        state: &RunState,
        interval: u64,
        executed: u64,
        last_snapshot_executed: u64,
    ) -> SnapshotWriteOutcome {
        // C1: Disabled → skip immediately.
        if interval == 0 {
            return SnapshotWriteOutcome::SkippedDisabled;
        }

        // C2: Check if enough steps have elapsed since the last snapshot.
        if executed.saturating_sub(last_snapshot_executed) < interval {
            return SnapshotWriteOutcome::SkippedNotReady;
        }

        // Resolve the underlying FjallJournal from the runtime journal trait.
        let Some(fjall) = self.journal.storage_journal() else {
            // Non-storage journal (volatile/noop): cannot write snapshots.
            // Best-effort: log and continue.
            return SnapshotWriteOutcome::SkippedNoStorage;
        };

        // Advance the journal sequence before attempting the write.
        // Per C-4: snapshot seq must be strictly greater than any journal event
        // seq written before the snapshot.
        let current_seq = self.journal_sequence_for(run);
        let snapshot_seq = match current_seq
            .get()
            .checked_add(1)
            .map(vb_storage::EventSeq::new)
        {
            Some(seq) => seq,
            None => {
                // Sequence overflow is a structural invariant violation.
                // Log and return Failed so the run continues.
                tracing::warn!(
                    "snapshot_write_failed seq_overflow run={}",
                    run.get()
                );
                return SnapshotWriteOutcome::Failed;
            }
        };

        // Serialise slots and taint from the RunFrame.
        // Serialization errors are non-fatal to the run lifecycle (C-2, C-5).
        let slots_bytes = match postcard::to_allocvec(&state.frame.slots_snapshot()) {
            Ok(bytes) => bytes,
            Err(_) => {
                tracing::warn!(
                    "snapshot_write_failed postcard_encode_failed_slots run={} seq={}",
                    run.get(),
                    snapshot_seq.get()
                );
                return SnapshotWriteOutcome::Failed;
            }
        };
        let taint_bytes = match postcard::to_allocvec(&state.frame.taint_snapshot()) {
            Ok(bytes) => bytes,
            Err(_) => {
                tracing::warn!(
                    "snapshot_write_failed postcard_encode_failed_taint run={} seq={}",
                    run.get(),
                    snapshot_seq.get()
                );
                return SnapshotWriteOutcome::Failed;
            }
        };

        let snapshot = RunSnapshot {
            run: state.frame.run_id(),
            seq: snapshot_seq,
            workflow: state.workflow.digest(),
            slots: slots_bytes,
            taint: taint_bytes,
        };

        // Attempt the write.
        let write_result = fjall.put_snapshot(&snapshot);

        match write_result {
            Ok(()) => {
                // Success: advance the journal sequence and update the threshold.
                self.journal_sequences.insert(run, snapshot_seq);
                // The caller updates last_snapshot_executed = executed via the
                // SnapshotWritten outcome.
                SnapshotWriteOutcome::Written
            }
            Err(_e) => {
                // Failure: roll back the sequence advance.
                self.journal_sequences.insert(run, current_seq);
                // Log the error per C-2 contract requirement.
                // The run continues without snapshot.
                tracing::warn!(
                    "snapshot_write_failed storage_write_error run={} seq={}",
                    run.get(),
                    snapshot_seq.get()
                );
                SnapshotWriteOutcome::Failed
            }
        }
    }
}

// =========================================================================
// `#[cfg(kani)]` stubs — avoid real I/O and complex types in Kani proofs
// =========================================================================

/// `#[cfg(kani)]` replacement for `write_snapshot_for_run`.
/// Kani proofs use a stub that always returns `SkippedDisabled`.
/// Trust boundary: TB-journal-stub-kani.
#[cfg(kani)]
impl Shard {
    pub(crate) fn write_snapshot_for_run(
        &mut self,
        _run: RunId,
        _state: &RunState,
        _interval: u64,
        _executed: u64,
        _last_snapshot_executed: u64,
    ) -> SnapshotWriteOutcome {
        SnapshotWriteOutcome::SkippedDisabled
    }
}

// =========================================================================
// RQ-W0-19: coalesce-buffer flush atomicity tests
// =========================================================================

#[cfg(all(test, not(kani)))]
#[allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used
)]
mod coalesce_atomicity_tests {
    use super::*;
    use crate::RuntimeError;
    use std::sync::{Arc, Mutex};

    /// Journal stub that fails the *first* batch attempt and succeeds on
    /// subsequent calls. Lets the test exercise the partial-failure path
    /// while keeping the success-path simple.
    #[derive(Debug)]
    struct FailFirstBatchJournal {
        events: Mutex<Vec<RuntimeJournalEvent>>,
        fail_remaining: Mutex<u32>,
    }

    impl FailFirstBatchJournal {
        fn shared(failures: u32) -> Arc<Self> {
            Arc::new(Self {
                events: Mutex::new(Vec::new()),
                fail_remaining: Mutex::new(failures),
            })
        }
        fn recorded(&self) -> Vec<RuntimeJournalEvent> {
            self.events
                .lock()
                .map(|v| v.clone())
                .unwrap_or_default()
        }
    }

    impl crate::journal::RuntimeJournal for FailFirstBatchJournal {
        fn append(&self, _event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            Ok(())
        }
        fn append_sequenced(
            &self,
            _event: RuntimeJournalEvent,
            _seq: EventSeq,
        ) -> crate::RuntimeResult<()> {
            Ok(())
        }
        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            _seq_start: EventSeq,
        ) -> crate::RuntimeResult<()> {
            let mut remaining = self
                .fail_remaining
                .lock()
                .map_err(|_| RuntimeError::JournalPoisoned)?;
            if *remaining > 0 {
                *remaining = remaining.saturating_sub(1);
                return Err(RuntimeError::StorageJournalAppend {
                    source: Arc::new(vb_storage::JournalError::QueueFull),
                });
            }
            drop(remaining);
            self.events
                .lock()
                .map_err(|_| RuntimeError::JournalPoisoned)?
                .extend_from_slice(events);
            Ok(())
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    fn step_started(run: RunId) -> RuntimeJournalEvent {
        RuntimeJournalEvent::StepStarted {
            run,
            step: vb_core::ids::StepIdx::ZERO,
        }
    }

    #[test]
    fn flush_failure_preserves_buffer_and_sequences() {
        let journal = FailFirstBatchJournal::shared(1);
        let mut shard = Shard::new_with_journal_and_artifact_store(
            ShardConfig {
                coalesce_window_ticks: 4,
                ..ShardConfig::default()
            },
            journal.clone(),
            crate::admission::AlwaysPresentArtifactStore::shared(),
        )
        .expect("shard must construct");

        let run = RunId::new(0xC0DE_19);

        // Buffer three events through the coalesce path.
        shard
            .append_journal_event(step_started(run))
            .expect("buffer event 1");
        shard
            .append_journal_event(step_started(run))
            .expect("buffer event 2");
        shard
            .append_journal_event(step_started(run))
            .expect("buffer event 3");

        // No events persisted yet — we are still inside the coalesce window.
        assert_eq!(
            journal.recorded().len(),
            0,
            "no events should be persisted while window is active"
        );
        assert_eq!(shard.coalesce_buffer.len(), 3);
        assert!(
            shard.journal_sequences.get(&run).is_none(),
            "journal_sequences must not advance while events are buffered (RQ-W0-19)"
        );

        // Force the coalesce window to expire by decrementing it to zero.
        shard.current_coalesce_window_remaining = 0;
        shard.tick().expect("tick must succeed");

        // The flush attempt failed: the buffer must remain intact so a
        // subsequent flush can retry the same events.
        assert_eq!(
            shard.coalesce_buffer.len(),
            3,
            "buffer must preserve events across a failed flush"
        );
        assert!(
            shard.journal_sequences.get(&run).is_none(),
            "journal_sequences must remain at zero after a failed flush"
        );
        assert_eq!(
            journal.recorded().len(),
            0,
            "failed flush must not record any events"
        );

        // Re-arm the window and force another flush; the retry succeeds and
        // advances the per-run sequence to exactly the buffered count.
        shard.current_coalesce_window_remaining = 0;
        shard.tick().expect("retry tick must succeed");

        assert_eq!(
            shard.coalesce_buffer.len(),
            0,
            "successful flush must drain the buffer"
        );
        assert_eq!(
            journal.recorded().len(),
            3,
            "retry must persist all three buffered events with the original starting sequence"
        );
        let seq_after = shard
            .journal_sequences
            .get(&run)
            .copied()
            .unwrap_or(EventSeq::ZERO);
        assert_eq!(
            seq_after.get(),
            3,
            "post-flush sequence must equal the buffered event count, not be double-advanced"
        );
    }
}
